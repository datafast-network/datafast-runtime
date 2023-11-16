use super::scylladb::Scylladb;
use super::RawEntity;
use crate::common::BlockPtr;
use crate::components::database::schema_lookup::SchemaLookup;
use crate::config::Config;
use crate::config::DatabaseConfig;
use crate::errors::DatabaseError;
use async_trait::async_trait;

pub enum ExternDB {
    Scylla(Scylladb),
}

impl ExternDB {
    pub async fn new(config: &Config, schema_lookup: SchemaLookup) -> Result<Self, DatabaseError> {
        let config = config.database.as_ref().unwrap();
        let db = match config {
            DatabaseConfig::Scylla { uri, keyspace } => {
                ExternDB::Scylla(Scylladb::new(uri, keyspace, schema_lookup).await?)
            }
        };

        Ok(db)
    }
}

#[async_trait]
pub trait ExternDBTrait: Sized {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError>;

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError>;

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError>;

    async fn load_entity_latest(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError>;

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>, //(entity_type, value)
    ) -> Result<(), DatabaseError>;

    async fn soft_delete_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError>;

    /// Revert all entity creations from given block ptr up to latest by hard-deleting them
    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError>;
}

#[async_trait]
impl ExternDBTrait for ExternDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.create_entity_tables().await,
        }
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.create_block_ptr_table().await,
        }
    }

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.load_entity(block_ptr, entity_type, entity_id).await,
        }
    }

    async fn load_entity_latest(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.load_entity_latest(entity_type, entity_id).await,
        }
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.create_entity(block_ptr, entity_type, data).await,
        }
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.batch_insert_entities(block_ptr, values).await,
        }
    }

    async fn soft_delete_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => {
                db.soft_delete_entity(block_ptr, entity_type, entity_id)
                    .await
            }
        }
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.revert_from_block(from_block).await,
        }
    }
}
