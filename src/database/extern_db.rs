use super::scylladb::Scylladb;
use crate::common::BlockPtr;
use crate::config::DatabaseConfig;
use crate::errors::DatabaseError;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;

#[derive(Default)]
pub enum ExternDB {
    Scylla(Scylladb),
    #[default]
    None,
}

//TODO: impl sql and mongodb
impl ExternDB {
    pub async fn new(
        config: &DatabaseConfig,
        schema_lookup: SchemaLookup,
    ) -> Result<Self, DatabaseError> {
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
        values: Vec<(EntityType, RawEntity)>,
    ) -> Result<(), DatabaseError>;

    async fn soft_delete_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), DatabaseError>;

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError>;

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError>;

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError>;

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError>;

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError>;

    async fn clean_up_data_history(&self, to_block: u64) -> Result<u64, DatabaseError>;
}

#[async_trait]
impl ExternDBTrait for ExternDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.create_entity_tables().await,
            ExternDB::None => Ok(()),
        }
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.create_block_ptr_table().await,
            ExternDB::None => Ok(()),
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
            ExternDB::None => Ok(None),
        }
    }

    async fn load_entity_latest(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.load_entity_latest(entity_type, entity_id).await,
            ExternDB::None => Ok(None),
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
            ExternDB::None => Ok(()),
        }
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.batch_insert_entities(block_ptr, values).await,
            ExternDB::None => Ok(()),
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

            ExternDB::None => Ok(()),
        }
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.revert_from_block(from_block).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.save_block_ptr(block_ptr).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.load_entities(entity_type, ids).await,
            ExternDB::None => Ok(vec![]),
        }
    }

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.load_recent_block_ptrs(number_of_blocks).await,
            ExternDB::None => Ok(vec![]),
        }
    }

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.get_earliest_block_ptr().await,
            ExternDB::None => Ok(None),
        }
    }

    async fn clean_up_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        match self {
            ExternDB::Scylla(db) => db.clean_up_data_history(to_block).await,
            ExternDB::None => Ok(1),
        }
    }
}
