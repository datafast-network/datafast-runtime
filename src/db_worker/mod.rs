use crate::config::Config;
use crate::errors::DatabaseWorkerError;
use crate::internal_messages::StoreOperationMessage;
use crate::internal_messages::StoreRequestResult;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
pub mod abstract_types;

type ImMemoryDatastore = HashMap<String, HashMap<String, HashMap<String, abstract_types::Value>>>;

type RawEntity = HashMap<String, abstract_types::Value>;

#[derive(Clone)]
pub enum DatabaseWorker {
    Memory(ImMemoryDatastore),
}

impl DatabaseWorker {
    pub fn handle_create(
        &mut self,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseWorkerError> {
        match self {
            Self::Memory(store) => {
                if !store.contains_key(&entity_type) {
                    store.insert(entity_type.clone(), HashMap::new());
                }

                let table = store.get_mut(&entity_type).unwrap();
                if let abstract_types::Value::String(entity_id) =
                    data.get("id").ok_or(DatabaseWorkerError::MissingID)?
                {
                    table.insert(entity_id.to_owned(), data);
                    Ok(())
                } else {
                    unimplemented!()
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Arc<RwLock<DatabaseWorker>>,
}

impl DatabaseWorker {
    pub async fn new(cfg: &Config) -> Result<Self, DatabaseWorkerError> {
        Ok(Self::Memory(HashMap::new()))
    }

    pub fn handle_request(
        &mut self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseWorkerError> {
        match request {
            StoreOperationMessage::Create(data) => {
                self.handle_create(data.0.clone(), data.1)?;
                Ok(StoreRequestResult::Create(data.0))
            }
            _ => Err(DatabaseWorkerError::Invalid),
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
    ) -> Result<StoreRequestResult, DatabaseWorkerError> {
        loop {
            match self.db.try_write() {
                Ok(mut db) => return db.handle_request(request),
                Err(_) => continue,
            }
        }
    }
}
