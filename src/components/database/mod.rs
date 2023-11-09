mod in_memory;

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
    Memory(in_memory::InMemoryDataStore),
}

pub trait DatabaseTrait {
    fn handle_create(&mut self, entity_type: String, data: RawEntity) -> Result<(), DatabaseError>;
    fn handle_load(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError>;
    fn handle_update(
        &mut self,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError>;
}

impl DatabaseTrait for Database {
    fn handle_create(&mut self, entity_type: String, data: RawEntity) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_create(entity_type, data),
        }
    }

    fn handle_load(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_load(entity_type, entity_id),
        }
    }

    fn handle_update(
        &mut self,
        entity_type: String,
        entity_id: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        match self {
            Self::Memory(store) => store.handle_update(entity_type, entity_id, data),
        }
    }
}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Arc<RwLock<Database>>,
}

impl Database {
    pub async fn new(_cfg: &Config) -> Result<Self, DatabaseError> {
        Ok(Self::Memory(HashMap::new()))
    }

    pub fn new_memory_db() -> Self {
        Self::Memory(HashMap::new())
    }

    pub fn handle_request(
        &mut self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        match request {
            StoreOperationMessage::Create(data) => {
                self.handle_create(data.0.clone(), data.1)?;
                Ok(StoreRequestResult::Create(data.0))
            }
            StoreOperationMessage::Load(data) => match self.handle_load(data.0, data.1)? {
                Some(e) => Ok(StoreRequestResult::Load(Some(e))),
                None => Ok(StoreRequestResult::Load(None)),
            },
            StoreOperationMessage::Update(data) => {
                assert!(!data.1.is_empty());
                self.handle_update(data.0, data.1, data.2)?;
                Ok(StoreRequestResult::Update)
            }
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn agent(&self) -> DatabaseAgent {
        DatabaseAgent {
            db: Arc::new(RwLock::new(self.to_owned())),
        }
    }
}

impl DatabaseAgent {
    pub fn send_store_request(
        self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        let mut db = self
            .db
            .try_write()
            .map_err(|_| DatabaseError::MutexLockFailed)?;
        db.handle_request(request)
    }
}

impl Default for DatabaseAgent {
    fn default() -> Self {
        Self {
            db: Arc::new(RwLock::new(Database::new_memory_db())),
        }
    }
}
