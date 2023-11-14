mod in_memory;
mod scylladb;

use crate::common::BlockPtr;
use crate::config::Config;
use crate::errors::DatabaseError;
use crate::messages::StoreOperationMessage;
use crate::messages::StoreRequestResult;
use crate::runtime::asc;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use self::in_memory::InMemoryDataStore;

type RawEntity = HashMap<String, asc::native_types::store::Value>;

#[derive(Clone)]
pub enum Database {
    Memory(Arc<Mutex<in_memory::InMemoryDataStore>>),
}

pub trait DatabaseTrait: Send + Sync {
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

    fn soft_delete(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), DatabaseError>;

    fn hard_delete(&mut self, entity_type: String, entity_id: String) -> Result<(), DatabaseError>;
}

impl DatabaseTrait for Database {
    fn handle_create(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => {
                store
                    .lock()
                    .unwrap()
                    .handle_create(block_ptr, entity_type, data)
            }
        }
    }

    fn handle_load(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            Self::Memory(store) => {
                store
                    .lock()
                    .unwrap()
                    .handle_load(block_ptr, entity_type, entity_id)
            }
        }
    }

    fn handle_load_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            Self::Memory(store) => store
                .lock()
                .unwrap()
                .handle_load_latest(entity_type, entity_id),
        }
    }

    fn soft_delete(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => {
                store
                    .lock()
                    .unwrap()
                    .soft_delete(block_ptr, entity_type, entity_id)
            }
        }
    }

    fn hard_delete(&mut self, entity_type: String, entity_id: String) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.lock().unwrap().hard_delete(entity_type, entity_id),
        }
    }
}

impl Database {
    pub async fn new(_cfg: &Config) -> Result<Self, DatabaseError> {
        Ok(Self::Memory(Arc::new(Mutex::new(
            InMemoryDataStore::default(),
        ))))
    }

    pub fn handle_wasm_host_request(
        &mut self,
        block_ptr: BlockPtr,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        match request {
            StoreOperationMessage::Load((entity_type, entity_id)) => {
                // When Wasm-Host ask for a load action, it is always ask for the latest snapshot
                match self.handle_load_latest(entity_type, entity_id)? {
                    Some(e) => Ok(StoreRequestResult::Load(Some(e))),
                    None => Ok(StoreRequestResult::Load(None)),
                }
            }
            StoreOperationMessage::Update((entity_type, _entity_id, data)) => {
                // Any Update request will always lead to a new snapshot creation
                self.handle_create(block_ptr, entity_type, data)?;
                Ok(StoreRequestResult::Update)
            }
            StoreOperationMessage::Delete((entity_type, entity_id)) => {
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
                self.soft_delete(block_ptr, entity_type, entity_id)?;
                Ok(StoreRequestResult::Delete)
            }
            _ => Err(DatabaseError::WasmSendInvalidRequest),
        }
    }

    pub fn agent(&self) -> DatabaseAgent {
        DatabaseAgent {
            db: self.clone(),
            block_ptr: None,
        }
    }
}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Database,
    pub block_ptr: Option<BlockPtr>,
}

impl Default for DatabaseAgent {
    fn default() -> Self {
        Self {
            db: Database::Memory(Arc::new(Mutex::new(InMemoryDataStore::default()))),
            block_ptr: None,
        }
    }
}

impl DatabaseAgent {
    pub fn wasm_send_store_request(
        mut self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        if self.block_ptr.is_none() {
            return Err(DatabaseError::MissingBlockPtr);
        }

        self.db
            .handle_wasm_host_request(self.block_ptr.to_owned().unwrap(), request)
    }
}
