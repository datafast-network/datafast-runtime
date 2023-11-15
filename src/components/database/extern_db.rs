use super::scylladb::Scylladb;
use super::RawEntity;
use crate::common::BlockPtr;
use crate::components::database::schema_lookup::SchemaLookup;
use crate::errors::DatabaseError;
use async_trait::async_trait;

pub(super) enum ExternDB {
    Scylla(Scylladb),
}

#[async_trait]
pub(super) trait ExternDBTrait: Sized {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError>;

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError>;

    fn get_schema_lockup(&self) -> &SchemaLookup;

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
