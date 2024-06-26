#[cfg(feature = "scylla")]
mod scylladb;
#[cfg(feature = "scylla")]
use scylladb::*;

#[cfg(feature = "mongo")]
mod mongo;
#[cfg(feature = "mongo")]
use mongo::*;

use crate::common::BlockPtr;
use crate::common::Datasource;
use crate::common::EntityID;
use crate::common::EntityType;
use crate::common::RawEntity;
use crate::common::Schemas;
use crate::config::DatabaseConfig;
use crate::errors::DatabaseError;
use async_trait::async_trait;

#[derive(Default)]
pub enum ExternDB {
    #[cfg(feature = "scylla")]
    Scylla(Scylladb),
    #[cfg(feature = "mongo")]
    Mongo(MongoDB),
    #[default]
    None,
}

impl ExternDB {
    pub async fn new(config: &DatabaseConfig, schemas: Schemas) -> Result<Self, DatabaseError> {
        let db = match config {
            #[cfg(feature = "scylla")]
            DatabaseConfig::Scylla { uri, keyspace } => {
                ExternDB::Scylla(Scylladb::new(uri, keyspace, schemas).await?)
            }
            #[cfg(feature = "mongo")]
            DatabaseConfig::Mongo { uri, database } => {
                ExternDB::Mongo(MongoDB::new(uri, database, schemas).await?)
            }
        };

        Ok(db)
    }
}

#[async_trait]
pub trait ExternDBTrait: Sized {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError>;

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError>;

    async fn create_datasource_table(&self) -> Result<(), DatabaseError>;

    async fn load_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError>;

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError>;

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError>;

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError>;

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError>;

    async fn save_datasources(&self, datasources: Vec<Datasource>) -> Result<(), DatabaseError>;

    async fn load_datasources(&self) -> Result<Option<Vec<Datasource>>, DatabaseError>;

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(EntityType, RawEntity)>,
    ) -> Result<(), DatabaseError>;

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError>;

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError>;

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError>;

    fn get_schema(&self) -> Schemas;
}

#[async_trait]
impl ExternDBTrait for ExternDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.create_entity_tables().await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.create_entity_tables().await,
            ExternDB::None => Ok(()),
        }
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.create_block_ptr_table().await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.create_block_ptr_table().await,
            ExternDB::None => Ok(()),
        }
    }

    async fn create_datasource_table(&self) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.create_datasource_table().await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.create_datasource_table().await,
            ExternDB::None => Ok(()),
        }
    }

    async fn load_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.load_entity(entity_type, entity_id).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.load_entity(entity_type, entity_id).await,
            ExternDB::None => Ok(None),
        }
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.load_entities(entity_type, ids).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.load_entities(entity_type, ids).await,
            ExternDB::None => Ok(vec![]),
        }
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.create_entity(block_ptr, entity_type, data).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.create_entity(block_ptr, entity_type, data).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.save_block_ptr(block_ptr).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.save_block_ptr(block_ptr).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.load_recent_block_ptrs(number_of_blocks).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.load_recent_block_ptrs(number_of_blocks).await,
            ExternDB::None => Ok(vec![]),
        }
    }

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.get_earliest_block_ptr().await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.get_earliest_block_ptr().await,
            ExternDB::None => Ok(None),
        }
    }

    async fn save_datasources(&self, datasources: Vec<Datasource>) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.save_datasources(datasources).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.save_datasources(datasources).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn load_datasources(&self) -> Result<Option<Vec<Datasource>>, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.load_datasources().await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.load_datasources().await,
            ExternDB::None => Ok(None),
        }
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(String, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.batch_insert_entities(block_ptr, values).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.batch_insert_entities(block_ptr, values).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.revert_from_block(from_block).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.revert_from_block(from_block).await,
            ExternDB::None => Ok(()),
        }
    }

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.remove_snapshots(entities, to_block).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.remove_snapshots(entities, to_block).await,
            ExternDB::None => Ok(0),
        }
    }

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.clean_data_history(to_block).await,
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.clean_data_history(to_block).await,
            ExternDB::None => Ok(1),
        }
    }

    fn get_schema(&self) -> Schemas {
        match self {
            #[cfg(feature = "scylla")]
            ExternDB::Scylla(db) => db.get_schema(),
            #[cfg(feature = "mongo")]
            ExternDB::Mongo(db) => db.get_schema(),
            ExternDB::None => Schemas::default(),
        }
    }
}
