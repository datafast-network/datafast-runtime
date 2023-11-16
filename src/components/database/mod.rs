use crate::common::BlockPtr;
use crate::errors::DatabaseError;
use crate::messages::StoreOperationMessage;
use crate::messages::StoreRequestResult;
use std::sync::Arc;
use tokio::sync::Mutex;

mod extern_db;
mod memory_db;
mod scylladb;

use crate::components::manifest_loader::SchemaLookup;
use crate::config::Config;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Value;
use extern_db::ExternDB;
use extern_db::ExternDBTrait;
use memory_db::MemoryDb;

pub struct Database {
    pub mem: MemoryDb,
    pub db: ExternDB,
}

impl Database {
    pub async fn new(config: &Config, schema_lookup: SchemaLookup) -> Result<Self, DatabaseError> {
        let mem = MemoryDb::default();
        let db = ExternDB::new(config, schema_lookup).await?;
        Ok(Database { mem, db })
    }

    async fn handle_store_request(
        &mut self,
        message: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        match message {
            StoreOperationMessage::Create(data) => self.handle_create(data).await,
            StoreOperationMessage::Load(data) => self.handle_load(data).await,
            StoreOperationMessage::Update(data) => self.handle_update(data).await,
            StoreOperationMessage::Delete(data) => self.handle_delete(data).await,
        }
    }

    async fn handle_create(
        &mut self,
        data: (EntityType, RawEntity),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, data) = data;
        let entity_id = data.get("id").cloned().expect("Missing ID in RawEntity");
        self.mem.create_entity(entity_type, data)?;

        if let Value::String(entity_id) = entity_id {
            Ok(StoreRequestResult::Create(entity_id))
        } else {
            Err(DatabaseError::InvalidValue("id is not string".to_string()))
        }
    }

    async fn handle_load(
        &mut self,
        data: (EntityType, EntityID),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, entity_id) = data;

        let entity = self
            .mem
            .load_entity_latest(entity_type.clone(), entity_id.clone())?;

        if entity.is_none() {
            let entity = self.db.load_entity_latest(&entity_type, &entity_id).await?;

            if entity.is_none() {
                return Ok(StoreRequestResult::Load(None));
            }

            let data = entity.unwrap();
            self.mem.create_entity(entity_type, data.clone())?;
            return Ok(StoreRequestResult::Load(Some(data)));
        }

        let data = entity.unwrap();
        Ok(StoreRequestResult::Load(Some(data)))
    }

    async fn handle_update(
        &mut self,
        data: (EntityType, EntityID, RawEntity),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, _entity_id, data) = data;
        self.handle_create((entity_type, data)).await?;
        Ok(StoreRequestResult::Update)
    }

    async fn handle_delete(
        &mut self,
        data: (EntityType, EntityID),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, entity_id) = data;
        self.mem.soft_delete(entity_type, entity_id)?;
        Ok(StoreRequestResult::Delete)
    }

    async fn migrate_from_mem_to_db(&mut self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let values = self.mem.extract_data()?;
        self.db
            .batch_insert_entities(block_ptr.clone(), values)
            .await?;
        self.db.save_block_ptr(block_ptr).await?;
        Ok(())
    }
}

// Draft
#[derive(Clone)]
pub struct Agent {
    db: Arc<Mutex<Database>>,
}

impl From<Database> for Agent {
    fn from(value: Database) -> Self {
        Self {
            db: Arc::new(Mutex::new(value)),
        }
    }
}

impl Agent {
    pub fn wasm_send_store_request(
        &self,
        message: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        let mut db = self.db.blocking_lock();
        let handle = tokio::runtime::Handle::current();
        handle.block_on(db.handle_store_request(message))
    }

    pub async fn migrate(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        db.migrate_from_mem_to_db(block_ptr).await
    }
    pub async fn clear_in_memory(&self) -> Result<(), DatabaseError> {
        self.db.lock().await.mem.clear();
        Ok(())
    }

    pub fn empty() -> Self {
        let mem = MemoryDb::default();
        let db = ExternDB::None;
        let database = Database { mem, db };
        Agent::from(database)
    }
}

#[macro_export]
macro_rules! schema {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        use $crate::components::manifest_loader::schema_lookup::FieldKind;
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), FieldKind{
            kind: $v,
            relation: None,
            list_inner_kind: None,
        }),)*]))
    }};
}

#[macro_export]
macro_rules! entity {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), $v),)*]))
    }};
}
