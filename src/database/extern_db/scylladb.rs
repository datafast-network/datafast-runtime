use super::ExternDBTrait;
use crate::common::BlockPtr;
use crate::debug;
use crate::error;
use crate::errors::DatabaseError;
use crate::info;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Bytes;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;
use crate::schema_lookup::FieldKind;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;
use futures_util::future::try_join_all;
use scylla::_macro_internal::CqlValue;
use scylla::batch::Batch;
use scylla::transport::session::Session;
use scylla::QueryResult;
use scylla::SessionBuilder;
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

impl From<Value> for CqlValue {
    fn from(value: Value) -> Self {
        match value {
            Value::String(str) => CqlValue::Text(str),
            Value::Int(int) => CqlValue::Int(int),
            Value::Int8(int8) => CqlValue::BigInt(int8),
            Value::BigDecimal(decimal) => CqlValue::Text(decimal.to_string()),
            Value::Bool(bool) => CqlValue::Boolean(bool),
            Value::List(list) => CqlValue::List(list.into_iter().map(CqlValue::from).collect()),
            Value::Bytes(bytes) => CqlValue::Blob(bytes.as_slice().to_vec()),
            Value::BigInt(n) => CqlValue::Text(n.to_string()),
            Value::Null => CqlValue::Empty,
        }
    }
}

#[derive(Clone)]
pub enum BlockPtrFilter {
    // Gt(u64),
    Gte(u64),
    Lt(u64),
    // Lte(u64),
}

impl Display for BlockPtrFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gte(block) => write!(f, "__block_ptr__ >= {block}"),
            Self::Lt(block) => write!(f, "__block_ptr__ < {block}"),
        }
    }
}

pub struct Scylladb {
    session: Arc<Session>,
    keyspace: String,
    schema_lookup: SchemaLookup,
}

impl Scylladb {
    pub async fn new(
        uri: &str,
        keyspace: &str,
        schema_lookup: SchemaLookup,
    ) -> Result<Self, DatabaseError> {
        info!(ExternDB, "Init db connection");
        let session: Session = SessionBuilder::new().known_node(uri).build().await?;
        let entities = schema_lookup.get_entity_names();
        let this = Self {
            session: Arc::new(session),
            keyspace: keyspace.to_owned(),
            schema_lookup,
        };
        this.create_keyspace().await?;
        info!(ExternDB, "Namespace created OK"; namespace => keyspace);
        this.create_entity_tables().await?;
        info!(ExternDB, "Entities table created OK"; entities => format!("{:?}", entities));
        this.create_block_ptr_table().await?;
        info!(ExternDB, "Block_Ptr table created OK");
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

    fn store_kind_to_db_type(field_kind: FieldKind) -> String {
        match field_kind.kind {
            StoreValueKind::Int => "int",
            StoreValueKind::Int8 => "bigint",
            StoreValueKind::String => "text",
            StoreValueKind::Bool => "boolean",
            StoreValueKind::BigDecimal => "text",
            StoreValueKind::BigInt => "text",
            StoreValueKind::Bytes => "blob",
            StoreValueKind::Array => {
                let inner_type = Scylladb::store_kind_to_db_type(FieldKind {
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

    fn cql_value_to_store_value(field_kind: FieldKind, value: Option<CqlValue>) -> Value {
        match field_kind.kind {
            StoreValueKind::Int => Value::Int(value.unwrap().as_int().unwrap()),
            StoreValueKind::Int8 => Value::Int8(value.unwrap().as_bigint().unwrap()),
            StoreValueKind::String => Value::String(value.unwrap().as_text().unwrap().to_owned()),
            StoreValueKind::Bool => Value::Bool(value.unwrap().as_boolean().unwrap()),
            StoreValueKind::BigDecimal => {
                Value::BigDecimal(BigDecimal::from_str(value.unwrap().as_text().unwrap()).unwrap())
            }
            StoreValueKind::BigInt => {
                Value::BigInt(BigInt::from_str(value.unwrap().as_text().unwrap()).unwrap())
            }
            StoreValueKind::Bytes => {
                let bytes_value = value.unwrap();
                let bytes = bytes_value.as_blob().unwrap();
                Value::Bytes(Bytes::from(bytes.as_slice()))
            }
            StoreValueKind::Array => {
                if value.is_none() {
                    return Value::List(vec![]);
                }
                let inner_values = value.unwrap().as_list().cloned().unwrap_or_default();
                let inner_values = inner_values
                    .into_iter()
                    .map(|inner_val| {
                        Scylladb::cql_value_to_store_value(
                            FieldKind {
                                kind: field_kind.list_inner_kind.unwrap(),
                                relation: None,
                                list_inner_kind: None,
                            },
                            Some(inner_val),
                        )
                    })
                    .collect::<Vec<_>>();
                Value::List(inner_values)
            }
            StoreValueKind::Null => unimplemented!(),
        }
    }

    fn handle_entity_query_result(
        &self,
        entity_type: &str,
        entity_query_result: QueryResult,
        include_deleted: bool,
    ) -> Vec<RawEntity> {
        let col_specs = entity_query_result.col_specs.clone();
        let rows = entity_query_result.rows().expect("Not a record-query");
        let mut result = vec![];

        for row in rows {
            let mut entity = RawEntity::new();
            for (idx, column) in row.columns.iter().enumerate() {
                let col_spec = col_specs[idx].clone();
                let field_name = col_spec.name.clone();
                let field_kind = self.schema_lookup.get_field(entity_type, &field_name);
                let value = Scylladb::cql_value_to_store_value(field_kind, column.clone());
                entity.insert(field_name, value);
            }

            let is_deleted = entity
                .get("__is_deleted__")
                .cloned()
                .expect("Missing `__is_deleted__` field");

            if is_deleted == Value::Bool(true) && !include_deleted {
                continue;
            }

            result.push(entity)
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
        let mut data_raw = data.clone();
        data_raw.insert("__is_deleted__".to_string(), Value::Bool(is_deleted));
        let (query, values) = self.generate_insert_query(entity_type, data_raw, block_ptr);
        self.session.query(query, values).await?;

        Ok(())
    }

    async fn get_ids_by_block_ptr_filter(
        &self,
        entity_type: &str,
        block_filter: &BlockPtrFilter,
    ) -> Result<HashSet<String>, DatabaseError> {
        let query = format!(
            r#"SELECT id FROM {}."{}" WHERE {}"#,
            self.keyspace, entity_type, block_filter
        );
        let rows = self.session.query(query, ()).await?.rows().unwrap();
        let ids = rows
            .into_iter()
            .map(|r| {
                r.columns
                    .first()
                    .cloned()
                    .unwrap()
                    .unwrap()
                    .into_string()
                    .unwrap()
            })
            .collect();

        Ok(ids)
    }

    #[cfg(test)]
    async fn drop_tables(&self) -> Result<(), DatabaseError> {
        let entities = self.schema_lookup.get_entity_names();
        for table_name in entities {
            let query = format!(r#"DROP TABLE IF EXISTS {}."{}""#, self.keyspace, table_name);
            self.session.query(query, ()).await?;
        }
        let query = format!(r#"DROP TABLE IF EXISTS {}.block_ptr"#, self.keyspace);
        self.session.query(query, ()).await?;
        Ok(())
    }

    #[cfg(test)]
    async fn soft_delete_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        let entity = self.load_entity(entity_type, entity_id).await?;

        if entity.is_none() {
            return Ok(());
        }

        let mut entity = entity.unwrap();
        entity.remove("__block_ptr__");
        entity.remove("__is_deleted__");

        self.insert_entity(block_ptr, entity_type, entity, true)
            .await
    }

    fn generate_insert_query(
        &self,
        entity_type: &str,
        data: RawEntity,
        block_ptr: BlockPtr,
    ) -> (String, Vec<CqlValue>) {
        let schema = self.schema_lookup.get_schema(entity_type);
        let mut fields: Vec<String> = vec![
            "\"__block_ptr__\"".to_string(),
            "\"__is_deleted__\"".to_string(),
        ];
        let mut column_values = vec!["?".to_string(), "?".to_string()];
        let mut values_params = vec![
            CqlValue::BigInt(block_ptr.number as i64),
            data.get("__is_deleted__").unwrap().clone().into(),
        ];
        for (field_name, field_kind) in schema.iter() {
            let value = match data.get(field_name) {
                None => {
                    //handle case when field is missing but has in schema
                    debug!(
                        Scylladb,
                        "Missing field";
                        entity_type => entity_type,
                        field_name => field_name,
                        data => format!("{:?}", data)
                    );
                    let default_value =
                        Scylladb::cql_value_to_store_value(field_kind.clone(), None);
                    CqlValue::from(default_value)
                }
                Some(val) => CqlValue::from(val.clone()),
            };
            values_params.push(value);
            fields.push(format!("\"{}\"", field_name));
            column_values.push("?".to_string());
        }

        assert_eq!(fields.len(), column_values.len());
        let joint_column_names = fields.join(",");
        let joint_column_values = column_values.join(",");

        let query = format!(
            r#"INSERT INTO {}."{}" ({}) VALUES ({})"#,
            self.keyspace, entity_type, joint_column_names, joint_column_values
        );

        (query, values_params)
    }
}

#[async_trait]
impl ExternDBTrait for Scylladb {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        let entities = self.schema_lookup.get_entity_names();
        for entity_type in entities {
            let schema = self.schema_lookup.get_schema(&entity_type);
            let mut column_definitions: Vec<String> = vec![];
            for (colum_name, store_kind) in schema.iter() {
                let column_type = Scylladb::store_kind_to_db_type(store_kind.clone());
                let definition = format!("\"{colum_name}\" {column_type}");
                column_definitions.push(definition);
            }
            // Add block_ptr
            column_definitions.push("__block_ptr__ bigint".to_string());

            // Add is_deleted for soft-delete
            column_definitions.push("__is_deleted__ boolean".to_string());

            // Define primary-key
            column_definitions.push("PRIMARY KEY (id, __block_ptr__)".to_string());

            let joint_column_definition = column_definitions.join(",\n");
            let query = format!(
                r#"CREATE TABLE IF NOT EXISTS {}."{}" (
            {joint_column_definition}
            ) WITH compression = {{'sstable_compression': 'LZ4Compressor'}} AND CLUSTERING ORDER BY (__block_ptr__ DESC)"#,
                self.keyspace, entity_type
            );
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
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let query = format!(
            r#"
            SELECT * from {}."{}"
            WHERE id = ?
            ORDER BY __block_ptr__ DESC
            LIMIT 1
            "#,
            self.keyspace, entity_type
        );

        let entity_query_result = self.session.query(query, (entity_id,)).await;
        match entity_query_result {
            Ok(result) => {
                let entity = self
                    .handle_entity_query_result(entity_type, result, false)
                    .first()
                    .cloned();
                Ok(entity)
            }
            Err(err) => {
                error!(ExternDB,
                    "Load entity latest error";
                    entity_type => entity_type,
                    entity_id => entity_id,
                    error => format!("{:?}", err)
                );
                Err(err.into())
            }
        }
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
        let mut inserts = vec![];
        let chunk_size = 100;
        let chunks = values.chunks(chunk_size);

        for chunk in chunks {
            let mut batch_queries = Batch::default();
            let mut batch_values = vec![];
            let session = self.session.clone();

            for (entity_type, data) in chunk.iter().cloned() {
                if data.get("__is_deleted__").is_none() {
                    error!(ExternDB,
                           "Missing is_deleted field";
                           entity_type => entity_type,
                           entity_data => format!("{:?}", data),
                           __block_ptr__ => block_ptr.number,
                           block_ptr_hash => block_ptr.hash
                    );
                    return Err(DatabaseError::MissingField("__is_deleted__".to_string()));
                }

                let (query, values) =
                    self.generate_insert_query(&entity_type, data, block_ptr.clone());
                batch_queries.append_statement(query.as_str());
                batch_values.push(values);
            }

            let st = session.prepare_batch(&batch_queries).await?;
            let insert = tokio::spawn(async move {
                Retry::spawn(ExponentialBackoff::from_millis(100), || {
                    session.batch(&st, batch_values.clone())
                })
                .await
            });

            inserts.push(insert);
        }

        let result = try_join_all(inserts).await.unwrap();
        info!(
            Scylladb,
            "Commit result";
            statements => format!("{:?} statements", result.len() * chunk_size),
            batch => format!("{:?} batches", result.len()),
            ok_batch => format!("{:?}", result.iter().filter(|r| r.is_ok()).collect::<Vec<_>>().len()),
            fail_batch => format!("{:?}", result.iter().filter(|r| r.is_err()).collect::<Vec<_>>())
        );

        Ok(())
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        let entity_names = self.schema_lookup.get_entity_names();
        let mut batch_queries: Batch = Batch::default();
        let mut batch_values = vec![];
        let block_ptr_filter = BlockPtrFilter::Gte(from_block);
        for entity_type in entity_names {
            let ids = self
                .get_ids_by_block_ptr_filter(&entity_type, &block_ptr_filter)
                .await?;
            for id in ids {
                let query = format!(
                    r#"
                    DELETE FROM {}."{}" WHERE id = ? AND {}"#,
                    self.keyspace, entity_type, block_ptr_filter
                );
                batch_queries.append_statement(query.as_str());
                batch_values.push((id,));
            }
        }
        let st_batch = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st_batch, batch_values).await?;
        Ok(())
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let partition_key = "dfr";
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
            SELECT * from {}."{}"
            WHERE id IN {}"#,
            self.keyspace, entity_type, ids
        );
        let entity_query_result = self.session.query(query, ()).await?;
        Ok(self.handle_entity_query_result(entity_type, entity_query_result, false))
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

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError> {
        let min_block_number = self
            .session
            .query(
                format!("SELECT min(block_number) FROM {}.block_ptr", self.keyspace),
                &[],
            )
            .await?;
        let row = min_block_number.first_row().unwrap();
        let column = row.columns.get(0).cloned().unwrap();

        if column.is_none() {
            return Ok(None);
        }

        let block_number = column.unwrap().as_bigint().unwrap() as u64;
        let query = format!(
            r#"
SELECT JSON block_number as number, block_hash as hash, parent_hash
FROM {}.block_ptr
WHERE sgd = ? AND block_number = {}"#,
            self.keyspace, block_number
        );
        let result = self.session.query(query, vec!["dfr".to_string()]).await?;
        let row = result.first_row().unwrap();
        let data = row.columns.get(0).cloned().unwrap();
        let text = data.unwrap().into_string().unwrap();
        return Ok(serde_json::from_str(&text).ok());
    }

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError> {
        let mut batch_queries: Batch = Batch::default();
        let mut batch_values = vec![];
        let block_ptr_filter = BlockPtrFilter::Lt(to_block);
        let mut count = 0;
        for (entity_name, entity_id) in entities {
            let query = format!(
                "DELETE FROM {}.\"{}\" WHERE id = ? AND {}",
                self.keyspace, entity_name, block_ptr_filter
            );
            batch_queries.append_statement(query.as_str());
            batch_values.push((entity_id,));
            count += 1;
        }

        let st_batch = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st_batch, batch_values).await?;
        Ok(count)
    }

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        let entity_names = self.schema_lookup.get_entity_names();
        let mut batch_queries: Batch = Batch::default();
        let mut batch_values = vec![];
        let block_ptr_filter = BlockPtrFilter::Lt(to_block);
        let mut count = 0;
        for entity_type in entity_names {
            let ids = self
                .get_ids_by_block_ptr_filter(&entity_type, &block_ptr_filter)
                .await?;
            count += ids.len();
            for id in ids {
                let query = format!(
                    r#"
                    DELETE FROM {}."{}" WHERE id = ? AND {}"#,
                    self.keyspace, entity_type, block_ptr_filter
                );
                batch_queries.append_statement(query.as_str());
                batch_values.push((id,));
            }
        }
        let query = format!(
            "DELETE FROM {}.block_ptr WHERE sgd = ? AND block_number < {to_block}",
            self.keyspace
        );
        batch_queries.append_statement(query.as_str());
        batch_values.push(("dfr".to_string(),));
        let st_batch = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st_batch, batch_values).await?;
        Ok(count as u64)
    }
}
