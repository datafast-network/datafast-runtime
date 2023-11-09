mod in_memmory;

use crate::common::BlockPtr;
use crate::config::Config;
use crate::errors::DatabaseError;
use crate::messages::StoreOperationMessage;
use crate::messages::StoreRequestResult;
use crate::runtime::asc;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

type RawEntity = HashMap<String, asc::native_types::store::Value>;

#[derive(Clone)]
pub enum Database {
    Memory(in_memmory::InMemoryDataStore),
}

pub trait DatabaseTrait {
    fn handle_create(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;
    fn handle_load(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError>;
    fn handle_load_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError>;
    fn handle_update(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;
    fn handle_update_latest(
        &mut self,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;
}

impl DatabaseTrait for Database {
    fn handle_create(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_create(block_ptr, entity_type, data),
        }
    }

    fn handle_load(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_load(block_ptr, entity_type, entity_id),
        }
    }

    fn handle_load_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_load_latest(entity_type, entity_id),
        }
    }

    fn handle_update(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_update(block_ptr, entity_type, entity_id, data),
        }
    }

    fn handle_update_latest(
        &mut self,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_update_latest(entity_type, entity_id, data),
        }
    }
}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Arc<RwLock<Database>>,
    pub block_ptr: Option<BlockPtr>,
}

impl Database {
    pub async fn new(_cfg: &Config) -> Result<Self, DatabaseError> {
        Ok(Self::Memory(HashMap::new()))
    }

    pub fn new_memory_db() -> Self {
        Self::Memory(HashMap::new())
    }

    pub fn handle_wasm_host_request(
        &mut self,
        block_ptr: BlockPtr,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        match request {
            StoreOperationMessage::Load(data) => {
                // When Wasm-Host ask for a load action, it is always ask for the latest snapshot
                match self.handle_load_latest(data.0, data.1)? {
                    Some(e) => Ok(StoreRequestResult::Load(Some(e))),
                    None => Ok(StoreRequestResult::Load(None)),
                }
            }
            StoreOperationMessage::Update(data) => {
                /* When wasm-host send an update request, that means user wants either
                 - To save a total new data record
                 - To update an existing data record
                So it is basically an Upsert request.
                Therefore, the flow to handle this should be:
                1/ Load latest entity with the given Entity-ID
                2/ If entity found, update it along with the new block-pointer
                - The new block-ptr might be a final block or not, we don't know
                - The easy way is to always create a new snapshot
                - The correct way is, find a way to know what is the current chain-head, compare it with the max-block-snapshot (it is quite like reorg threshold).
                ---> If (chain-head - block-ptr) > max-block-snapshots, it means we are out of reorg threshold, we can mutate the current data, or, insert the new row, and delete the old row
                ---> Else, just insert the new row
                >>>> Conclusion: we always should create a new snapshot,
                then later we shall determine if we need to remove the previous snapshot or not.
                3/ If entity not found, create it with the current block-pointer
                */
                assert!(!data.1.is_empty());
                self.handle_update(block_ptr, data.0, data.1, data.2)?;
                Ok(StoreRequestResult::Update)
            }
            StoreOperationMessage::Delete(data) => {
                /*
                - If we are out of reorg-threshold, we can safely HARD-DELETE all snapshots of this Entity
                - If we are within the reorg-threshold, we can only SOFT-DELETE all the snapshots
                - If reorg happen, how do we know if the soft-delete action should be reverted or not?
                - To handle this, we can make SOFT-DELETE column a Numeric value, and when soft-delete happens,
                we add block-ptr's block-number to the SOFT-DELETE column
                - When reorg happen, we know the reorg-block, then...
                - If the reorg-block is > SOFT-DELETE's block, we do nothing
                - Else, we clear the SOFT-DELETE column
                */
                todo!()
            }
            _ => Err(DatabaseError::WasmSendInvalidRequest),
        }
    }

    pub fn agent(&self) -> DatabaseAgent {
        DatabaseAgent {
            db: Arc::new(RwLock::new(self.to_owned())),
            block_ptr: None,
        }
    }
}

impl DatabaseAgent {
    pub fn wasm_send_store_request(
        self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        let mut db = self
            .db
            .try_write()
            .map_err(|_| DatabaseError::MutexLockFailed)?;

        if self.block_ptr.is_none() {
            return Err(DatabaseError::MissingBlockPtr);
        }

        db.handle_wasm_host_request(self.block_ptr.to_owned().unwrap(), request)
    }
}

impl Default for DatabaseAgent {
    fn default() -> Self {
        Self {
            db: Arc::new(RwLock::new(Database::new_memory_db())),
            block_ptr: None,
        }
    }
}
