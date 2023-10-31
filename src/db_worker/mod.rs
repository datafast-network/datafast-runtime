use crate::config::Config;
use crate::errors::DatabaseWorkerError;
use crate::internal_messages::StoreOperationMessage;
use crate::internal_messages::StoreRequestResult;
use std::sync::Arc;
use std::sync::RwLock;
pub mod abstract_types;

#[derive(Clone)]
pub struct DatabaseWorker {}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Arc<RwLock<DatabaseWorker>>,
}

impl DatabaseWorker {
    pub async fn new(cfg: &Config) -> Result<Self, DatabaseWorkerError> {
        todo!()
    }

    pub fn handle_request(
        &mut self,
        request: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseWorkerError> {
        todo!()
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
