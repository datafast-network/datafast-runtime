use super::extern_db::ExternDB;
use super::memory_db::MemoryDb;
use super::schema_lookup::SchemaLookup;
use crate::config::Config;
use crate::errors::DatabaseError;
use crate::messages::EntityType;
use crate::messages::StoreOperationMessage;
use crate::messages::StoreRequestResult;
use crate::runtime::asc::native_types::store::Value;
use std::collections::HashMap;

pub struct Database2 {
    pub mem: MemoryDb,
    pub db: ExternDB,
}

impl Database2 {
    pub async fn new(config: &Config, schema_lookup: SchemaLookup) -> Result<Self, DatabaseError> {
        let mem = MemoryDb::default();
        let db = ExternDB::new(config, schema_lookup).await?;
        Ok(Database2 { mem, db })
    }

    pub async fn handle_store_request(
        &mut self,
        message: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        match message {
            StoreOperationMessage::Create(data) => self.handle_create(data).await,
            StoreOperationMessage::Load(data) => unimplemented!(),
            StoreOperationMessage::Update(_) => unimplemented!(),
            StoreOperationMessage::Delete(_) => unimplemented!(),
        }
    }

    async fn handle_create(
        &mut self,
        data: (EntityType, HashMap<String, Value>),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, data) = data;
        let entity_id = data.get("id").cloned().unwrap();
        self.mem.create_entity(entity_type, data)?;
        if let Value::String(entity_id) = entity_id {
            Ok(StoreRequestResult::Create(entity_id))
        } else {
            Err(DatabaseError::InvalidValue("id".to_string()))
        }
    }
}
