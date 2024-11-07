use std::str::FromStr;

use super::ExternDBTrait;
use crate::runtime::asc::native_types::store::Bytes as StoreBytes;
use crate::runtime::asc::native_types::store::Value as StoreValue;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;

use crate::{
    common::{BlockPtr, Datasource, EntityID, EntityType, FieldKind, RawEntity, Schemas},
    errors::DatabaseError,
    runtime::asc::native_types::store::StoreValueKind,
};
use async_trait::async_trait;
use df_logger::info;
use sqlx::postgres::PgRow;
use sqlx::Row;

pub struct PostgresDB {
    conn: sqlx::PgPool,
    schemas: Schemas,
    pg_schema: String,
    chain_id: Option<String>,
}

impl PostgresDB {
    pub async fn new(
        uri: &str,
        schemas: Schemas,
        pg_schema: &str,
        chain_id: Option<String>,
    ) -> Result<Self, DatabaseError> {
        let conn = sqlx::PgPool::connect(uri)
            .await
            .map_err(DatabaseError::PostgresErr)?;
        Ok(Self {
            conn,
            schemas,
            pg_schema: pg_schema.to_string(),
            chain_id,
        })
    }

    fn store_kind_to_db_type(field_kind: &FieldKind) -> String {
        match field_kind.kind {
            StoreValueKind::Int => "int",
            StoreValueKind::Int8 => "bigint",
            StoreValueKind::String => "text",
            StoreValueKind::Bool => "boolean",
            StoreValueKind::BigDecimal => "text",
            StoreValueKind::BigInt => "text",
            StoreValueKind::Bytes => "bytea",
            StoreValueKind::Array => {
                let inner_type = Self::store_kind_to_db_type(&FieldKind {
                    kind: field_kind.list_inner_kind.unwrap(),
                    relation: None,
                    list_inner_kind: None,
                });
                return format!("{}[]", inner_type);
            }
            StoreValueKind::Null => unimplemented!(),
        }
        .to_string()
    }

    fn column_to_store_valu(
        value: serde_json::Value,
        field_name: &str,
        field_kind: StoreValueKind,
    ) -> StoreValue {
        match field_kind {
            StoreValueKind::Int => StoreValue::Int(i32::from(value.as_i64().unwrap())),
            StoreValueKind::Int8 => StoreValue::Int8(value.as_i64().unwrap()),
            StoreValueKind::String => StoreValue::String(value.as_str().unwrap().to_string()),
            StoreValueKind::Bool => StoreValue::Bool(row.get::<bool, _>(field_name)),
            StoreValueKind::BigDecimal => StoreValue::BigDecimal(
                BigDecimal::from_str(value.as_str().unwrap().to_string()).unwrap(),
            ),
            StoreValueKind::BigInt => {
                StoreValue::BigInt(BigInt::from_str(row.get::<&str, _>(field_name)).unwrap())
            }
            StoreValueKind::Bytes => {
                let raw_bytes = row.get::<&[u8], _>(field_name);
                let bytes = StoreBytes::from(raw_bytes.to_vec());
                StoreValue::Bytes(bytes)
            }
            StoreValueKind::Array => {
                todo!("Array not implemented")
            }
            StoreValueKind::Null => unimplemented!(),
        }
    }

    fn row_to_raw_entity(schemas: &Schemas, entity_type: &str, row: PgRow) -> RawEntity {
        let mut result = RawEntity::new();
        let schema = schemas.get_schema(entity_type);

        for (idx, (field_name, field_kind)) in schema.into_iter().enumerate() {
            match field_kind.kind {
                StoreValueKind::Bytes => {
                    let raw_bytes = row.get::<&[u8], _>(field_name.as_str());
                    let bytes = StoreBytes::from(raw_bytes.to_vec());
                    result.insert(field_name.clone(), StoreValue::Bytes(bytes));
                    continue;
                }
                _ => {
                    let value = row.get::<serde_json::Value, _>(field_name.as_str());
                    todo!()
                }
            }

            // let column = row.get(idx);
            // let field_value = StoreValue::try_from((column, &field_kind.kind)).unwrap();
            // result.insert(field_name.clone(), field_value);
        }
        result
    }
}

#[async_trait]
impl ExternDBTrait for PostgresDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        let table_names = self.schemas.get_entity_names();

        for table_name in table_names {
            let schema = self.schemas.get_schema(&table_name);

            let mut column_definitions: Vec<String> = vec![];
            for (colum_name, store_kind) in schema.iter() {
                let column_type = Self::store_kind_to_db_type(store_kind);
                let definition = format!("\"{colum_name}\" {column_type}");
                column_definitions.push(definition);
                info!(
                    PostgresDB,
                    "Column definition";
                    column_name => colum_name.clone(),
                    column_type => column_type.clone()
                );
            }

            // Define primary-key
            if self.chain_id.is_some() {
                // Add chain_id
                column_definitions.push("__chain_id__ text".to_string());
                column_definitions
                    .push("PRIMARY KEY (id, __block_ptr__, __chain_id__)".to_string());
            } else {
                column_definitions.push("PRIMARY KEY (id, __block_ptr__)".to_string());
            }

            let joint_column_definition = column_definitions.join(",\n");
            let query = format!(
                r#"CREATE TABLE IF NOT EXISTS {}."{}" (
            {joint_column_definition}
            )"#,
                self.pg_schema, table_name
            );
            info!(
                PostgresDB,
                "Table creation query";
                query => query.clone()
            );
            sqlx::query(&query)
                .fetch_all(&self.conn)
                .await
                .map_err(DatabaseError::PostgresErr)?;
        }

        Ok(())
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        if self.chain_id.is_some() {
            let query = format!(
                r#"CREATE TABLE IF NOT EXISTS {}.__block_ptr__ (
            block_number bigint NOT NULL,
            block_hash text NOT NULL,
            parent_hash text NOT NULL,
            chain_id text NOT NULL,
            PRIMARY KEY (block_number, chain_id)
        )"#,
                self.pg_schema
            );
            info!(
                PostgresDB,
                "Block ptr table creation query";
                query => query.clone()
            );
            sqlx::query(&query)
                .fetch_all(&self.conn)
                .await
                .map_err(DatabaseError::PostgresErr)?;
            return Ok(());
        }

        let query = format!(
            r#"CREATE TABLE IF NOT EXISTS {}.__block_ptr__ (
            block_number bigint NOT NULL,
            block_hash text NOT NULL,
            parent_hash text NOT NULL,
        )"#,
            self.pg_schema
        );
        info!(
            PostgresDB,
            "Block ptr table creation query";
            query => query.clone()
        );
        sqlx::query(&query)
            .fetch_all(&self.conn)
            .await
            .map_err(DatabaseError::PostgresErr)?;
        Ok(())
    }

    async fn create_datasource_table(&self) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn load_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        if self.chain_id.is_some() {
            let query = format!(
                r#"SELECT * FROM {}.{} WHERE id = $1 AND __chain_id__ = $2 ORDER BY __block_ptr__ DESC LIMIT 1"#,
                self.pg_schema, entity_type
            );

            let row = sqlx::query(&query)
                .bind(entity_id)
                .bind(self.chain_id.as_ref().unwrap())
                .fetch_one(&self.conn)
                .await
                .map_err(DatabaseError::PostgresErr)?;
        } else {
            let query = format!(
                r#"SELECT * FROM {}.{} WHERE id = $1 ORDER BY __block_ptr__ DESC LIMIT 1"#,
                self.pg_schema, entity_type
            );
            let row = sqlx::query(&query)
                .bind(entity_id)
                .fetch_one(&self.conn)
                .await
                .map_err(DatabaseError::PostgresErr)?;
        };

        todo!()
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        todo!()
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        todo!()
    }

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError> {
        todo!()
    }

    async fn save_datasources(&self, datasources: Vec<Datasource>) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn load_datasources(&self) -> Result<Option<Vec<Datasource>>, DatabaseError> {
        todo!()
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(EntityType, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError> {
        todo!()
    }

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        todo!()
    }

    fn get_schema(&self) -> Schemas {
        self.schemas.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use df_logger::loggers::init_logger;
    use std::fs::read_to_string;

    #[tokio::test]
    async fn test_create_entity_table() {
        init_logger();

        let gql =
            read_to_string("../subgraph-testing/packages/v0_0_5/build/schema.graphql").unwrap();

        let schemas = Schemas::new_from_graphql_schema(&gql);

        let db = PostgresDB::new(
            "postgres://postgres:postgres@localhost:5432/postgres",
            schemas,
            "public",
            Some("0x1".to_string()),
        )
        .await
        .unwrap();

        db.create_entity_tables().await.unwrap();

        db.create_block_ptr_table().await.unwrap();

        db.create_datasource_table().await.unwrap();
    }
}
