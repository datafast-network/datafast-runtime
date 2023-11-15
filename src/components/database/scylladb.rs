use super::extern_db::ExternDBTrait;
use super::schema_lookup::SchemaLookup;
use super::RawEntity;
use crate::common::BlockPtr;
use crate::error;
use crate::errors::DatabaseError;
use crate::info;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use async_trait::async_trait;
use scylla::transport::session::Session;
use scylla::SessionBuilder;
use scylla::_macro_internal::ValueList;
use scylla::batch::Batch;
use scylla::query::Query;
use std::collections::HashMap;

pub struct Scylladb {
    session: Session,
    keyspace: String,
    schema_lookup: SchemaLookup,
}

impl Scylladb {
    pub(super) async fn new(uri: &str, keyspace: &str) -> Result<Self, DatabaseError> {
        let session: Session = SessionBuilder::new().known_node(uri).build().await?;
        let this = Self {
            session,
            keyspace: keyspace.to_owned(),
            schema_lookup: SchemaLookup::default(),
        };
        Ok(this)
    }

    #[cfg(test)]
    async fn create_test_keyspace(&self) -> Result<(), DatabaseError> {
        let q = format!(
            r#"
                CREATE KEYSPACE IF NOT EXISTS {} WITH REPLICATION = {{'class' : 'NetworkTopologyStrategy', 'replication_factor' : 1}}
            "#,
            self.keyspace
        );
        self.session.query(q, []).await?;
        self.session
            .query(format!("USE {}", self.keyspace), [])
            .await?;
        Ok(())
    }

    fn store_kind_to_db_type(&self, kind: StoreValueKind) -> String {
        match kind {
            StoreValueKind::Int => "int",
            StoreValueKind::Int8 => "bigint",
            StoreValueKind::String => "text",
            StoreValueKind::Bool => "boolean",
            StoreValueKind::BigDecimal => "text",
            StoreValueKind::BigInt => "text",
            StoreValueKind::Bytes => "blob",
            StoreValueKind::Array => unimplemented!(),
            StoreValueKind::Null => unimplemented!(),
        }
        .to_string()
    }

    async fn get_entity(
        &self,
        query: impl Into<Query>,
        values: impl ValueList,
        entity_name: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let result = self.session.query(query, values).await?;

        match result.single_row() {
            Ok(row) => {
                let json_row = row.columns.first().cloned().unwrap().unwrap();
                let json_row_as_str = json_row.as_text().unwrap();
                let json: serde_json::Value = serde_json::from_str(json_row_as_str).unwrap();
                if let serde_json::Value::Object(values) = json {
                    let result = self.schema_lookup.json_to_entity(entity_name, values);
                    Ok(Some(result))
                } else {
                    error!(Scylladb, "Not an json object"; data => json);
                    Err(DatabaseError::Invalid)
                }
            }
            Err(error) => {
                error!(Scylladb, "Error when get entity"; error => error);
                Err(DatabaseError::InvalidValue(error.to_string()))
            }
        }
    }

    async fn get_entities(
        &self,
        query: impl Into<Query>,
        values: impl ValueList,
        entity_name: &str,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        let result = self.session.query(query, values).await?;

        match result.rows() {
            Ok(rows) => {
                let mut entities = vec![];
                for row in rows {
                    let json_row = row.columns.first().cloned().unwrap().unwrap();
                    let json_row_as_str = json_row.as_text().unwrap();
                    let json: serde_json::Value = serde_json::from_str(json_row_as_str).unwrap();
                    if let serde_json::Value::Object(values) = json {
                        let result = self.schema_lookup.json_to_entity(entity_name, values);
                        entities.push(result);
                    } else {
                        error!(Scylladb, "Not an json object"; data => json);
                        continue;
                    };
                }
                Ok(entities)
            }
            Err(e) => {
                error!(Scylladb, "Error when get entities"; error => e);
                Err(DatabaseError::InvalidValue(e.to_string()))
            }
        }
    }

    async fn insert_record(
        &self,
        block_ptr: BlockPtr,
        entity_name: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        assert!(data.contains_key("id"));
        let mut json_data = self.schema_lookup.entity_to_json(entity_name, data);

        json_data.insert(
            "block_ptr_number".to_string(),
            serde_json::Value::from(block_ptr.number),
        );

        json_data.insert(
            "block_ptr_hash".to_string(),
            serde_json::Value::from(block_ptr.hash),
        );

        json_data.insert("is_deleted".to_string(), serde_json::Value::Bool(false));

        let json_data = serde_json::Value::Object(json_data);

        let query = format!("INSERT INTO {}.{} JSON ?", self.keyspace, entity_name);

        info!(Scylladb, "created-entity query"; query => query);
        let st = self.session.prepare(query).await?;
        self.session
            .execute(&st, vec![json_data.to_string()])
            .await?;

        Ok(())
    }
}

#[async_trait]
impl ExternDBTrait for Scylladb {
    async fn create_entity_table(
        &self,
        entity_type: &str,
        schema: HashMap<String, StoreValueKind>,
    ) -> Result<(), DatabaseError> {
        // let query = format!("create table `{entity_type}` ",);

        let mut column_definitions: Vec<String> = vec![];

        for (colum_name, store_kind) in schema {
            let column_type = self.store_kind_to_db_type(store_kind);
            let definition = format!("{colum_name} {column_type}");
            column_definitions.push(definition);
        }

        // Add block_ptr
        column_definitions.push("block_ptr_number bigint".to_string());
        column_definitions.push("block_ptr_hash text".to_string());

        // Add is_deleted for soft-delete
        column_definitions.push("is_deleted boolean".to_string());

        // Define primary-key
        column_definitions.push("PRIMARY KEY (id, block_ptr_number, block_ptr_hash)".to_string());

        let joint_column_definition = column_definitions.join(",\n");

        let query = format!(
            r#"CREATE TABLE IF NOT EXISTS {}.{entity_type} (
            {joint_column_definition}
            ) WITH compression = {{'sstable_compression': 'LZ4Compressor'}} AND CLUSTERING ORDER BY (block_ptr_number DESC)"#,
            self.keyspace
        );

        info!(Scylladb, "create-table query"; query => query);

        self.session.query(query, &[]).await?;

        Ok(())
    }

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let raw_query = format!(
            r#"
            SELECT JSON * from {}.{}
            WHERE block_ptr_number = ? AND block_ptr_hash = ? AND id = ? AND is_deleted = false
            LIMIT 1 ALLOW FILTERING
            "#,
            self.keyspace, entity_type
        );
        info!(Scylladb, "load entity"; query => raw_query);
        let result = self
            .get_entity(
                raw_query,
                (block_ptr.number as i64, block_ptr.hash, entity_id),
                entity_type,
            )
            .await?;

        Ok(result)
    }

    async fn load_entities(&self, entity_type: &str) -> Result<Vec<RawEntity>, DatabaseError> {
        let raw_query = format!(
            r#"
            SELECT JSON * from {}.{}
            WHERE is_deleted = false ALLOW FILTERING
            "#,
            self.keyspace, entity_type
        );
        self.get_entities(raw_query, (), entity_type).await
    }

    async fn load_entity_latest(
        &self,
        entity_name: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let raw_query = format!(
            r#"
            SELECT JSON * from {}.{}
            WHERE id = ? AND is_deleted = false
            ORDER BY block_ptr_number DESC
            LIMIT 1 ALLOW FILTERING
            "#,
            self.keyspace, entity_name
        );
        let result = self
            .get_entity(raw_query, (entity_id,), entity_name)
            .await?;
        Ok(result)
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_name: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        self.insert_record(block_ptr, entity_name, data).await
    }

    async fn create_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        //TODO: batch insert
        for (entity_name, data) in values {
            self.insert_record(block_ptr.clone(), &entity_name, data)
                .await?;
        }
        Ok(())
    }

    async fn soft_delete_entity(
        &self,
        entity_name: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        let latest = self.load_entity_latest(entity_name, entity_id).await?;

        if latest.is_none() {
            return Ok(());
        }

        let mut latest = latest.unwrap();
        *latest.get_mut("is_deleted").unwrap() = Value::Bool(true);

        let block_ptr_number =
            if let Value::Int8(n) = latest.get("block_ptr_number").cloned().unwrap() {
                n as u64
            } else {
                unimplemented!()
            };

        let block_ptr_hash =
            if let Value::String(n) = latest.get("block_ptr_hash").cloned().unwrap() {
                n
            } else {
                unimplemented!()
            };

        let query = format!(
            r#"
            UPDATE {}.{} SET is_deleted = True
            WHERE id = ? AND block_ptr_number = ? AND block_ptr_hash = ?"#,
            self.keyspace, entity_name
        );
        let st = self.session.prepare(query).await?;
        self.session
            .execute(&st, (entity_id, block_ptr_number as i64, block_ptr_hash))
            .await?;

        Ok(())
    }

    async fn hard_delete_entity(
        &self,
        entity_types: Vec<String>,
        from_block: u64,
    ) -> Result<(), DatabaseError> {
        let mut batch_queries: Batch = Batch::default();
        let mut batch_values = vec![];
        for entity_name in entity_types {
            let query = format!(
                r#"
                SELECT id from {}.{}
                WHERE block_ptr_number >= ?"#,
                self.keyspace, entity_name
            );
            let result = self.session.query(query, (from_block as i64,)).await?;
            let mut ids = vec![];
            if let Ok(rows) = result.rows() {
                for row in rows {
                    let row_json = row.columns.first().cloned().unwrap().unwrap();
                    let json_row_as_str = row_json.as_text().unwrap();
                    ids.push(json_row_as_str.clone())
                }
            }
            for id in ids {
                let query = format!(
                    r#"
                    DELETE FROM {}.{} WHERE id = ? AND block_ptr_number >= ?"#,
                    self.keyspace, entity_name,
                );
                batch_queries.append_statement(query.as_str());
                batch_values.push((id, from_block as i64));
            }
        }
        let st_batch = self.session.prepare_batch(&batch_queries).await?;
        self.session.batch(&st_batch, batch_values).await?;
        Ok(())
    }

    /// Revert all entity creations from given block ptr up to latest by hard-deleting them
    async fn revert_create_entity(&self, from_block: u64) -> Result<(), DatabaseError> {
        // Get all schemas
        let table_names = self.schema_lookup.get_entity_names();
        self.hard_delete_entity(table_names, from_block).await?;
        Ok(())
    }

    /// Revert all entity deletion from given block ptr up to latest by nullifying `is_deleted` fields
    async fn revert_delete_entity(&self, _from_block: u64) -> Result<(), DatabaseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::ExternDBTrait;
    use super::*;
    use crate::entity;
    use crate::runtime::asc::native_types::store::Value;
    use crate::runtime::bignumber::bigint::BigInt;
    use env_logger;
    use log::info;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_scylla_create_and_load_entity() {
        env_logger::try_init().unwrap_or_default();

        let uri = "localhost:9042";
        let keyspace = "ks";
        let mut db = Scylladb::new(uri, keyspace).await.unwrap();

        db.schema_lookup.add_schema(
            "Tokens",
            entity!(
                id => StoreValueKind::String,
                name => StoreValueKind::String,
                symbol => StoreValueKind::String,
                total_supply => StoreValueKind::BigInt,
                block_ptr_number => StoreValueKind::Int8,
                block_ptr_hash => StoreValueKind::String,
                is_deleted => StoreValueKind::Bool,
            ),
        );

        db.create_test_keyspace().await.unwrap();
        info!("Create KEYSPACE OK!");

        db.create_entity_table(
            "Tokens",
            entity!(
                id => StoreValueKind::String,
                name => StoreValueKind::String,
                symbol => StoreValueKind::String,
                total_supply => StoreValueKind::BigInt,
            ),
        )
        .await
        .unwrap();
        info!("Create TABLE OK!");

        let entity_data = entity! {
            id => Value::String("token-id".to_string()),
            name => Value::String("Tether USD".to_string()),
            symbol => Value::String("USDT".to_string()),
            total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap())
        };

        let block_ptr = BlockPtr::default();

        db.create_entity(block_ptr.clone(), "Tokens", entity_data)
            .await
            .unwrap();
        info!("Create test Token OK!");

        let loaded_entity = db
            .load_entity(block_ptr.clone(), "Tokens", "token-id")
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
            .load_entity_latest("Tokens", "token-id")
            .await
            .unwrap()
            .unwrap();

        info!("Loaded-latest from db: {:?}", loaded_entity);
        assert_eq!(
            loaded_entity.get("id").cloned(),
            Some(Value::String("token-id".to_string()))
        );

        // ------------------------------- Soft delete
        info!("Test soft delete");
        db.soft_delete_entity("Tokens", "token-id").await.unwrap();
        info!("soft delete done");
        assert!(db.load_entity_latest("Tokens", "token-id").await.is_err());
    }
    #[tokio::test]
    async fn test_scylla_revert_db() {
        env_logger::try_init().unwrap_or_default();

        let uri = "localhost:9042";
        let keyspace = "ks";
        let mut db = Scylladb::new(uri, keyspace).await.unwrap();
        db.schema_lookup.add_schema(
            "Tokens",
            entity!(
                id => StoreValueKind::String,
                name => StoreValueKind::String,
                symbol => StoreValueKind::String,
                total_supply => StoreValueKind::BigInt,
                block_ptr_number => StoreValueKind::Int8,
                block_ptr_hash => StoreValueKind::String,
                is_deleted => StoreValueKind::Bool,
            ),
        );

        db.create_test_keyspace().await.unwrap();
        info!("Create KEYSPACE OK!");
        //insert records
        for id in 1..10 {
            let entity_data = entity! {
                id => Value::String(format!("token-id_{}", id)),
                name => Value::String("Tether USD".to_string()),
                symbol => Value::String("USDT".to_string()),
                total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap())
            };
            let block_ptr = BlockPtr {
                number: id,
                hash: format!("hash_{}", id),
            };

            db.create_entity(block_ptr.clone(), "Tokens", entity_data)
                .await
                .unwrap();
        }

        //revert
        db.revert_create_entity(5).await.unwrap();

        let entities = db.load_entities("Tokens").await.unwrap();

        assert_eq!(entities.len(), 4);
    }
}
