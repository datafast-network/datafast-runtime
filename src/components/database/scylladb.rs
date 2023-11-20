use super::extern_db::ExternDBTrait;
use crate::common::BlockPtr;
use crate::components::manifest_loader::schema_lookup::FieldKind;
use crate::components::manifest_loader::schema_lookup::SchemaLookup;
use crate::error;
use crate::errors::DatabaseError;
use crate::info;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use async_trait::async_trait;
use scylla::batch::Batch;
use scylla::transport::session::Session;
use scylla::QueryResult;
use scylla::SessionBuilder;

pub struct Scylladb {
    session: Session,
    keyspace: String,
    schema_lookup: SchemaLookup,
}

impl Scylladb {
    pub async fn new(
        uri: &str,
        keyspace: &str,
        schema_lookup: SchemaLookup,
    ) -> Result<Self, DatabaseError> {
        let session: Session = SessionBuilder::new().known_node(uri).build().await?;
        let this = Self {
            session,
            keyspace: keyspace.to_owned(),
            schema_lookup,
        };
        this.create_keyspace().await?;
        Ok(this)
    }

    async fn create_keyspace(&self) -> Result<(), DatabaseError> {
        let q = format!(
            r#"
                CREATE KEYSPACE IF NOT EXISTS {} WITH REPLICATION = {{'class' : 'NetworkTopologyStrategy', 'replication_factor' : 1}}
            "#,
            self.keyspace
        );
        self.session.query(q, []).await?;
        Ok(())
    }

    fn store_kind_to_db_type(&self, field_kind: FieldKind) -> String {
        match field_kind.kind {
            StoreValueKind::Int => "int",
            StoreValueKind::Int8 => "bigint",
            StoreValueKind::String => "text",
            StoreValueKind::Bool => "boolean",
            StoreValueKind::BigDecimal => "text",
            StoreValueKind::BigInt => "text",
            StoreValueKind::Bytes => "blob",
            StoreValueKind::Array => {
                let inner_type = self.store_kind_to_db_type(FieldKind {
                    kind: field_kind.list_inner_kind.unwrap(),
                    relation: None,
                    list_inner_kind: None,
                });
                return format!("list<{}>", inner_type);
            }
            StoreValueKind::Null => unimplemented!(),
        }
        .to_string()
    }

    fn handle_entity_query_result(
        &self,
        entity_type: &str,
        entity_query_result: QueryResult,
        include_deleted: bool,
    ) -> Vec<RawEntity> {
        let rows = entity_query_result.rows().expect("Not a record-query");
        let mut result = vec![];

        for row in rows.into_iter() {
            let json_as_first_column = row.columns.first().cloned().unwrap().unwrap();
            let json_as_str = json_as_first_column.as_text().unwrap();
            let actual_json: serde_json::Value = serde_json::from_str(json_as_str).unwrap();

            if let serde_json::Value::Object(values) = actual_json {
                let object = self.schema_lookup.json_to_entity(entity_type, values);
                let is_deleted = object
                    .get("is_deleted")
                    .cloned()
                    .expect("Missing `is_deleted` field");

                if is_deleted == Value::Bool(true) && !include_deleted {
                    continue;
                }

                result.push(object)
            }
        }

        result
    }

    async fn insert_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
        is_deleted: bool,
    ) -> Result<(), DatabaseError> {
        assert!(data.contains_key("id"));
        let mut json_data = self.schema_lookup.entity_to_json(entity_type, data);

        json_data.insert(
            "block_ptr_number".to_string(),
            serde_json::Value::from(block_ptr.number),
        );

        json_data.insert(
            "is_deleted".to_string(),
            serde_json::Value::Bool(is_deleted),
        );

        let json_data = serde_json::Value::Object(json_data);

        let query = format!(
            "INSERT INTO {}.{} JSON ?",
            self.keyspace,
            entity_type.to_lowercase()
        );

        let st = self.session.prepare(query).await?;
        self.session
            .execute(&st, vec![json_data.to_string()])
            .await?;

        Ok(())
    }

    async fn get_ids(
        &self,
        entity_type: &str,
        block_range: (i64, Option<i64>),
    ) -> Result<Vec<String>, DatabaseError> {
        let mut ids = vec![];
        let query = match block_range.1 {
            Some(stop_block) => format!(
                r#"
                SELECT id from {}.{}
                WHERE block_ptr_number >= {} AND block_ptr_number <= {}"#,
                self.keyspace,
                entity_type.to_lowercase(),
                block_range.0,
                stop_block
            ),
            None => format!(
                r#"
                SELECT id from {}.{}
                WHERE block_ptr_number >= {}"#,
                self.keyspace,
                entity_type.to_lowercase(),
                block_range.0
            ),
        };
        let result = self.session.query(query, ()).await?;

        if let Ok(rows) = result.rows() {
            for row in rows {
                let row_json = row.columns.first().cloned().unwrap().unwrap();
                let json_row_as_str = row_json.as_text().unwrap();
                ids.push(json_row_as_str.clone())
            }
        }
        Ok(ids)
    }

    #[cfg(test)]
    async fn drop_tables(&self) -> Result<(), DatabaseError> {
        let schema = self.schema_lookup.get_schemas();
        for (table_name, _) in schema.iter() {
            let query = format!("DROP TABLE IF EXISTS {}.\"{}\"", self.keyspace, table_name);
            self.session.query(query, ()).await?;
        }
        let query = format!("DROP TABLE IF EXISTS {}.block_ptr", self.keyspace);
        self.session.query(query, ()).await?;
        Ok(())
    }
}

#[async_trait]
impl ExternDBTrait for Scylladb {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        for (entity_type, schema) in self.schema_lookup.get_schemas() {
            let mut column_definitions: Vec<String> = vec![];
            for (colum_name, store_kind) in schema.iter() {
                let column_type = self.store_kind_to_db_type(store_kind.clone());
                let definition = format!("\"{colum_name}\" {column_type}");
                column_definitions.push(definition);
            }
            // Add block_ptr
            column_definitions.push("block_ptr_number bigint".to_string());

            // Add is_deleted for soft-delete
            column_definitions.push("is_deleted boolean".to_string());

            // Define primary-key
            column_definitions.push("PRIMARY KEY (id, block_ptr_number)".to_string());

            let joint_column_definition = column_definitions.join(",\n");
            let query = format!(
                r#"CREATE TABLE IF NOT EXISTS {}."{}" (
            {joint_column_definition}
            ) WITH compression = {{'sstable_compression': 'LZ4Compressor'}} AND CLUSTERING ORDER BY (block_ptr_number DESC)"#,
                self.keyspace,
                entity_type.to_lowercase()
            );
            info!(Scylladb, "Creating entity table"; query => query);
            self.session.query(query, &[]).await?;
        }

        Ok(())
    }

    /// For Scylla DB, block_ptr table has to use the same primary `sgd` value for all row so the table can be properly sorted,
    /// Though anti-pattern, we only need to change the prefix if the block_ptr table
    /// grows too big to be stored in a single db node
    /// TODO: we can dynamically config this prefix later
    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.block_ptr (
                sgd text,
                block_number bigint,
                block_hash text,
                parent_hash text,
                PRIMARY KEY (sgd, block_number)
            ) WITH compression = {{'sstable_compression': 'LZ4Compressor'}} AND CLUSTERING ORDER BY (block_number DESC)
"#,
            self.keyspace
        );
        self.session.query(query, ()).await?;
        Ok(())
    }

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let query = format!(
            r#"
                SELECT JSON * from {}.{}
                WHERE block_ptr_number = ? AND id = ?
                LIMIT 1
            "#,
            self.keyspace, entity_type
        );
        let entity_query_result = self
            .session
            .query(query, (block_ptr.number as i64, entity_id))
            .await?;
        let entity = self
            .handle_entity_query_result(entity_type, entity_query_result, false)
            .first()
            .cloned();
        Ok(entity)
    }

    async fn load_entity_latest(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let query = format!(
            r#"
            SELECT JSON * from {}.{}
            WHERE id = ?
            ORDER BY block_ptr_number DESC
            LIMIT 1
            "#,
            self.keyspace, entity_type
        );

        let entity_query_result = self.session.query(query, (entity_id,)).await?;
        let entity = self
            .handle_entity_query_result(entity_type, entity_query_result, false)
            .first()
            .cloned();
        Ok(entity)
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        self.insert_entity(block_ptr, entity_type, data, false)
            .await
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        let mut batch_queries = Batch::default();
        let mut batch_values = vec![];
        for (entity_type, data) in values {
            if data.get("is_deleted").is_none() {
                error!(Scylladb,
                    "Missing is_deleted field";
                    entity_type => entity_type,
                    entity_data => format!("{:?}", data),
                    block_ptr_number => block_ptr.number,
                    block_ptr_hash => block_ptr.hash
                );
                return Err(DatabaseError::MissingField("is_deleted".to_string()));
            }

            let mut json_data = self.schema_lookup.entity_to_json(&entity_type, data);
            json_data.insert(
                "block_ptr_number".to_string(),
                serde_json::Value::from(block_ptr.number),
            );

            let data_json: String = serde_json::Value::Object(json_data).to_string();
            let query = format!("INSERT INTO {}.{} JSON ?", self.keyspace, entity_type);

            batch_queries.append_statement(query.as_str());
            batch_values.push((data_json,))
        }
        let st = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st, batch_values).await?;
        Ok(())
    }

    async fn soft_delete_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        let entity = self.load_entity_latest(entity_type, entity_id).await?;

        if entity.is_none() {
            return Ok(());
        }

        let mut entity = entity.unwrap();
        entity.remove("block_ptr_number");
        entity.remove("is_deleted");

        self.insert_entity(block_ptr, entity_type, entity, true)
            .await
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        let entity_names = self.schema_lookup.get_entity_names();
        let mut batch_queries: Batch = Batch::default();
        let mut batch_values = vec![];
        for entity_type in entity_names {
            let ids = self
                .get_ids(&entity_type, (from_block as i64, None))
                .await?;
            for id in ids {
                let query = format!(
                    r#"
                    DELETE FROM {}."{}" WHERE id = ? AND block_ptr_number >= ?"#,
                    self.keyspace,
                    entity_type.to_lowercase(),
                );
                batch_queries.append_statement(query.as_str());
                batch_values.push((id, from_block as i64));
            }
        }
        let st_batch = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st_batch, batch_values).await?;
        Ok(())
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let partition_key = "swr";
        let query = format!(
            r#"
            INSERT INTO {}.block_ptr (sgd, block_number, block_hash, parent_hash) VALUES ('{partition_key}', ?, ?, ?)"#,
            self.keyspace
        );
        self.session
            .query(
                query,
                (
                    block_ptr.number as i64,
                    block_ptr.hash,
                    block_ptr.parent_hash,
                ),
            )
            .await?;
        Ok(())
    }

    fn get_schema(&self) -> SchemaLookup {
        self.schema_lookup.clone()
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        let ids = format!(
            "({})",
            ids.into_iter()
                .map(|e| format!("'{}'", e))
                .collect::<Vec<_>>()
                .join(",")
        );
        let query = format!(
            r#"
            SELECT JSON * from {}.{}
            WHERE id IN {}"#,
            self.keyspace, entity_type, ids
        );
        let entity_query_result = self.session.query(query, ()).await?;
        Ok(self.handle_entity_query_result(&entity_type, entity_query_result, false))
    }

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        let query = format!(
            "SELECT JSON block_number as number, block_hash as hash, parent_hash FROM {}.block_ptr LIMIT {};",
            self.keyspace, number_of_blocks
        );
        let result = self.session.query(query, &[]).await?;

        if let Ok(mut rows) = result.rows() {
            let block_ptrs = rows
                .iter_mut()
                .rev()
                .filter_map(|r| {
                    let json = r
                        .columns
                        .first()
                        .cloned()
                        .unwrap()
                        .unwrap()
                        .as_text()
                        .cloned()
                        .unwrap();
                    serde_json::from_str::<BlockPtr>(&json).ok()
                })
                .collect::<Vec<_>>();

            return Ok(block_ptrs);
        }

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::ExternDBTrait;
    use super::*;
    use crate::entity;
    use crate::runtime::asc::native_types::store::Value;
    use crate::runtime::bignumber::bigint::BigInt;
    use crate::schema;
    use env_logger;
    use log::info;
    use std::collections::HashMap;
    use std::str::FromStr;

    async fn setup_db(entity_type: &str) -> (Scylladb, String) {
        env_logger::try_init().unwrap_or_default();

        let uri = "localhost:9042";
        let keyspace = "ks";
        let mut schema = SchemaLookup::new();

        let mut test_schema: HashMap<String, FieldKind> = schema!(
            id => StoreValueKind::String,
            name => StoreValueKind::String,
            symbol => StoreValueKind::String,
            total_supply => StoreValueKind::BigInt,
            userBalance => StoreValueKind::BigInt,
            tokenBlockNumber => StoreValueKind::BigInt,
            users => StoreValueKind::Array,
            table => StoreValueKind::String
        );

        test_schema.get_mut("users").unwrap().list_inner_kind = Some(StoreValueKind::String);

        schema.add_schema(entity_type, test_schema);
        let db = Scylladb::new(uri, keyspace, schema).await.unwrap();
        db.drop_tables().await.unwrap();
        db.create_entity_tables()
            .await
            .expect("Create entity tables failed");
        db.create_block_ptr_table()
            .await
            .expect("Create block-ptr table failed");
        db.revert_from_block(0).await.expect("Revert table failed");
        (db, entity_type.to_string())
    }

    #[tokio::test]
    async fn test_scylla_01_setup_db() {
        setup_db("test").await;
    }

    #[tokio::test]
    async fn test_scylla_02_create_and_load_entity() {
        let (db, entity_type) = setup_db("Tokens_01").await;

        let entity_data: RawEntity = entity! {
            id => Value::String("token-id".to_string()),
            name => Value::String("Tether USD".to_string()),
            symbol => Value::String("USDT".to_string()),
            total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap()),
            userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
            tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
            users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
            table => Value::String("dont-matter".to_string())
        };

        let block_ptr = BlockPtr::default();

        db.create_entity(block_ptr.clone(), &entity_type, entity_data.clone())
            .await
            .unwrap();

        info!("Create test Token OK!");

        let loaded_entity = db
            .load_entity(block_ptr.clone(), &entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();

        info!("Load test Token OK!");
        info!("Loaded from db: {:?}", loaded_entity);
        assert_eq!(
            loaded_entity.get("id").cloned(),
            Some(Value::String("token-id".to_string()))
        );
        assert_eq!(
            loaded_entity.get("name").cloned(),
            Some(Value::String("Tether USD".to_string()))
        );
        assert_eq!(
            loaded_entity.get("symbol").cloned(),
            Some(Value::String("USDT".to_string()))
        );
        assert_eq!(
            loaded_entity.get("total_supply").cloned(),
            Some(Value::BigInt(
                BigInt::from_str("111222333444555666777888999").unwrap()
            ))
        );
        assert_eq!(
            loaded_entity.get("is_deleted").cloned(),
            Some(Value::Bool(false))
        );

        // ------------------------------- Load latest
        let loaded_entity = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();

        info!("Loaded-latest from db: {:?}", loaded_entity);
        assert_eq!(
            loaded_entity.get("id").cloned(),
            Some(Value::String("token-id".to_string()))
        );

        let block_ptr = BlockPtr {
            number: 1,
            hash: "hash_1".to_string(),
            parent_hash: "".to_string(),
        };
        db.create_entity(block_ptr.clone(), &entity_type, entity_data)
            .await
            .unwrap();

        let loaded_entity = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();

        info!("Loaded-latest from db: {:?}", loaded_entity);
        assert_eq!(
            loaded_entity.get("id").cloned(),
            Some(Value::String("token-id".to_string()))
        );
        assert_eq!(
            loaded_entity.get("block_ptr_number").cloned(),
            Some(Value::Int8(1))
        );
    }
    //
    #[tokio::test]
    async fn test_scylla_03_revert_entity() {
        let (db, entity_type) = setup_db("Tokens_03").await;

        for id in 0..10 {
            let entity_data = entity! {
                id => Value::String("token-id".to_string()),
                name => Value::String("Tether USD".to_string()),
                symbol => Value::String("USDT".to_string()),
                total_supply => Value::BigInt(BigInt::from(id*1000)),
                userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
                tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
                users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
                table => Value::String("dont-matter".to_string()),
                is_deleted => Value::Bool(false)
            };
            let block_ptr = BlockPtr {
                number: id,
                hash: format!("hash_{}", id),
                parent_hash: "".to_string(),
            };

            db.create_entity(block_ptr.clone(), &entity_type, entity_data)
                .await
                .unwrap();
        }

        let latest = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.get("block_ptr_number"), Some(&Value::Int8(9)));

        db.revert_from_block(5).await.unwrap();

        let latest = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.get("block_ptr_number"), Some(&Value::Int8(4)));

        db.soft_delete_entity(
            BlockPtr {
                number: 5,
                hash: "hash".to_string(),
                parent_hash: "".to_string(),
            },
            &entity_type,
            "token-id",
        )
        .await
        .unwrap();

        let latest = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap();
        assert!(latest.is_none());

        db.revert_from_block(3).await.unwrap();

        let latest = db
            .load_entity_latest(&entity_type, "token-id")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.get("block_ptr_number"), Some(&Value::Int8(2)));
    }

    #[tokio::test]
    async fn test_scylla_04_batch_insert() {
        let (db, entity_type) = setup_db("Tokens_04").await;

        let mut entities = Vec::new();
        let block_ptr = BlockPtr {
            number: 0,
            hash: "hash".to_string(),
            parent_hash: "parent_hash1".to_string(),
        };

        let mut ids = Vec::new();

        for id in 0..10 {
            let entity_data: RawEntity = entity! {
                id => Value::String(format!("token-id_{}", id)),
                name => Value::String("Tether USD".to_string()),
                symbol => Value::String("USDT".to_string()),
                total_supply => Value::BigInt(BigInt::from(id*1000)),
                userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
                tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
                users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
                table => Value::String("dont-matter".to_string()),
                is_deleted => Value::Bool(id % 2 == 0)
            };
            ids.push(format!("token-id_{}", id));
            entities.push((entity_type.clone(), entity_data));
        }

        db.batch_insert_entities(block_ptr.clone(), entities)
            .await
            .unwrap();

        let entities_values = db.load_entities(&entity_type, ids).await.unwrap();

        assert_eq!(entities_values.len(), 5);

        let latest = db
            .load_entity_latest(&entity_type, "token-id_0")
            .await
            .unwrap();

        assert!(latest.is_none());

        let latest = db
            .load_entity_latest(&entity_type, "token-id_1")
            .await
            .unwrap();

        assert!(latest.is_some());

        let latest = latest.unwrap();
        assert_eq!(
            latest.get("total_supply"),
            Some(&Value::BigInt(BigInt::from(1000)))
        );
    }

    #[tokio::test]
    async fn test_scylla_05_get_relation() {
        env_logger::try_init().unwrap_or_default();

        let uri = "localhost:9042";
        let keyspace = "ks";
        let mut schema = SchemaLookup::new();
        let entity_type = "test_relation";
        let tokens = "tokens_relation";
        let mut entity_1: HashMap<String, FieldKind> = HashMap::new();
        entity_1.insert(
            "id".to_string(),
            FieldKind {
                kind: StoreValueKind::String,
                relation: None,
                list_inner_kind: None,
            },
        );
        entity_1.insert(
            "name".to_string(),
            FieldKind {
                kind: StoreValueKind::String,
                relation: None,
                list_inner_kind: None,
            },
        );
        entity_1.insert(
            "token_id".to_string(),
            FieldKind {
                kind: StoreValueKind::Array,
                relation: Some((tokens.to_string(), "id".to_string())),
                list_inner_kind: Some(StoreValueKind::String),
            },
        );
        schema.add_schema(entity_type, entity_1);

        let mut entity_2: HashMap<String, FieldKind> = HashMap::new();
        entity_2.insert(
            "id".to_string(),
            FieldKind {
                kind: StoreValueKind::String,
                relation: None,
                list_inner_kind: None,
            },
        );
        entity_2.insert(
            "name".to_string(),
            FieldKind {
                kind: StoreValueKind::String,
                relation: None,
                list_inner_kind: None,
            },
        );
        schema.add_schema(tokens, entity_2);

        let db = Scylladb::new(uri, keyspace, schema).await.unwrap();
        db.drop_tables().await.unwrap();
        db.create_entity_tables().await.unwrap();

        let block_ptr = BlockPtr {
            number: 0,
            hash: "hash".to_string(),
            parent_hash: "".to_string(),
        };
        for token in 0..5 {
            let token_entity: RawEntity = entity! {
                id => Value::String(format!("token-id_{}", token)),
                name => Value::String(format!("token-name_{}", token)),
            };
            db.insert_entity(block_ptr.clone(), tokens, token_entity, false)
                .await
                .unwrap();
        }

        let mut entity_data: RawEntity = entity! {
            id => Value::String(format!("entity-id_{}", 1)),
            name => Value::String("entity-name".to_string()),
        };
        entity_data.insert(
            "token_id".to_string(),
            Value::List(vec![
                Value::String("token-id_0".to_string()),
                Value::String("token-id_1".to_string()),
                Value::String("token-id_2".to_string()),
            ]),
        );

        db.insert_entity(block_ptr.clone(), entity_type, entity_data, false)
            .await
            .unwrap();

        let latest = db
            .load_entity_latest(entity_type, "entity-id_1")
            .await
            .unwrap()
            .unwrap();
        let relations = latest.get("token_id").cloned().unwrap();
        let relation_ids = match relations {
            Value::List(list) => {
                let mut relation = vec![];
                list.iter().for_each(|value| {
                    if let Value::String(entity_id) = value {
                        relation.push(entity_id.clone())
                    }
                });
                relation
            }
            _ => panic!("Not a list"),
        };
        log::info!("relation: {:?}", relation_ids);
        let tokens_relation = db.load_entities(&tokens, relation_ids).await.unwrap();

        assert_eq!(tokens_relation.len(), 3);
    }

    #[tokio::test]
    async fn test_scylla_06_save_load_block_ptr() {
        let (db, _entity_name) = setup_db("Tokens_06").await;

        for i in 7..12 {
            db.save_block_ptr(BlockPtr {
                number: i,
                hash: format!("hash-{i}"),
                parent_hash: format!("parent-hash-{i}"),
            })
            .await
            .unwrap();
        }

        let number_of_blocks = 10;
        let recent_block_ptrs = db.load_recent_block_ptrs(number_of_blocks).await.unwrap();

        assert_eq!(recent_block_ptrs.len(), 5);
        assert_eq!(recent_block_ptrs.last().cloned().unwrap().number, 11);
        assert_eq!(recent_block_ptrs.first().cloned().unwrap().number, 7);
    }
}
