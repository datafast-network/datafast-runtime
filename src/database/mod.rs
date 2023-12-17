mod extern_db;
mod memory_db;
mod metrics;
mod utils;

use crate::common::BlockPtr;
use crate::common::Datasource;
use crate::common::EntityID;
use crate::common::EntityType;
use crate::common::FieldName;
use crate::common::RawEntity;
use crate::common::StoreOperationMessage;
use crate::common::StoreRequestResult;
use crate::config::DatabaseConfig;
use crate::errors::DatabaseError;
use crate::info;
use crate::runtime::asc::native_types::store::Value;
use crate::schema_lookup::SchemaLookup;
use crate::warn;
use extern_db::ExternDB;
use extern_db::ExternDBTrait;
use memory_db::MemoryDb;
use metrics::DatabaseMetrics;
use prometheus::Registry;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub struct Database {
    pub mem: MemoryDb,
    pub db: ExternDB,
    pub earliest_block: u64,
    metrics: DatabaseMetrics,
    schema: SchemaLookup,
}

impl Database {
    pub async fn new(
        config: &DatabaseConfig,
        schema: SchemaLookup,
        registry: &Registry,
    ) -> Result<Self, DatabaseError> {
        let mem = MemoryDb::default();
        let db = ExternDB::new(config, schema.clone()).await?;
        let earliest_block = db
            .get_earliest_block_ptr()
            .await?
            .map(|b| b.number)
            .unwrap_or(0);
        let metrics = DatabaseMetrics::new(registry);
        Ok(Database {
            mem,
            db,
            metrics,
            schema,
            earliest_block,
        })
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
            StoreOperationMessage::LoadRelated(data) => self.handle_load_related(data).await,
            StoreOperationMessage::LoadInBlock(data) => self.handle_load_in_block(data),
        }
    }

    async fn handle_create(
        &mut self,
        data: (EntityType, RawEntity),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, data) = data;
        let entity_id = data.get("id").cloned().expect("Missing ID in RawEntity");
        self.mem.create_entity(&entity_type, data)?;

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

        let entity = self.mem.load_entity_latest(&entity_type, &entity_id)?;

        if entity.is_none() {
            self.metrics.cache_miss.inc();
            self.metrics.extern_db_load.inc();
            let entity = self.db.load_entity(&entity_type, &entity_id).await?;

            if entity.is_none() {
                return Ok(StoreRequestResult::Load(None));
            }

            let data = entity.unwrap();
            self.mem.create_entity(&entity_type, data.clone())?;
            return Ok(StoreRequestResult::Load(Some(data)));
        }

        self.metrics.cache_hit.inc();
        let data = entity.unwrap();
        Ok(StoreRequestResult::Load(Some(data)))
    }

    fn handle_load_in_block(
        &self,
        data: (EntityType, EntityID),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, entity_id) = data;
        let entity = self.mem.load_entity_latest(&entity_type, &entity_id)?;
        Ok(StoreRequestResult::Load(entity))
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
        self.mem.soft_delete(&entity_type, &entity_id)?;
        Ok(StoreRequestResult::Delete)
    }

    async fn handle_load_related(
        &mut self,
        data: (EntityType, EntityID, FieldName),
    ) -> Result<StoreRequestResult, DatabaseError> {
        let (entity_type, entity_id, field_name) = data;
        let entity = self.mem.load_entity_latest(&entity_type, &entity_id)?;

        //In memory always have the latest version of the entity by action load before.
        //We don't need to check the db

        let entity = entity.unwrap();
        let field_related_ids = entity.get(&field_name).cloned().unwrap();
        let ids = match field_related_ids {
            Value::String(id) => vec![id],
            Value::List(list) => {
                let mut ids = vec![];
                list.iter().for_each(|value| {
                    if let Value::String(entity_id) = value {
                        ids.push(entity_id.clone())
                    }
                });
                ids
            }
            _ => vec![],
        };

        if let Some((relation_table, _field_name)) =
            self.schema.get_relation_field(&entity_type, &field_name)
        {
            let mut related_entities = vec![];
            let mut missing_ids = vec![];
            for id in ids {
                let entity = self.mem.load_entity_latest(&relation_table, &id)?;
                if entity.is_some() {
                    related_entities.push(entity.unwrap());
                } else {
                    missing_ids.push(id);
                }
            }
            if !missing_ids.is_empty() {
                let entities = self.db.load_entities(&relation_table, missing_ids).await?;
                for entity in entities {
                    related_entities.push(entity.clone());
                    self.mem.create_entity(&relation_table, entity)?;
                }
            }
            Ok(StoreRequestResult::LoadRelated(related_entities))
        } else {
            Ok(StoreRequestResult::LoadRelated(vec![]))
        }
    }

    async fn migrate_from_mem_to_db(&mut self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let values = self.mem.extract_data()?;
        self.metrics.extern_db_write.inc();
        self.db
            .batch_insert_entities(block_ptr.clone(), values)
            .await?;
        self.metrics.extern_db_write.inc();
        self.db.save_block_ptr(block_ptr.clone()).await?;
        Ok(())
    }

    async fn revert_from_block(&mut self, block_number: u64) -> Result<(), DatabaseError> {
        self.mem.clear();
        self.db.revert_from_block(block_number).await
    }
}

#[derive(Clone)]
pub struct DatabaseAgent {
    db: Arc<RwLock<Database>>,
}

impl From<Database> for DatabaseAgent {
    fn from(value: Database) -> Self {
        Self {
            db: Arc::new(RwLock::new(value)),
        }
    }
}

impl DatabaseAgent {
    pub async fn new(
        config: &DatabaseConfig,
        schema: SchemaLookup,
        registry: &Registry,
    ) -> Result<Self, DatabaseError> {
        let db = Database::new(config, schema.to_owned(), registry).await?;
        Ok(Self::from(db))
    }

    pub async fn load_datasources(&self) -> Result<Option<Vec<Datasource>>, DatabaseError> {
        self.db.read().await.db.load_datasources().await
    }

    pub async fn save_datasources(&self, ds: Vec<Datasource>) -> Result<(), DatabaseError> {
        self.db.write().await.db.save_datasources(ds).await
    }

    pub fn wasm_send_store_request(
        &self,
        message: StoreOperationMessage,
    ) -> Result<StoreRequestResult, DatabaseError> {
        use std::thread;
        let db = self.db.clone();

        thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut db = db.write().await;
                let result = db.handle_store_request(message).await?;
                Ok::<StoreRequestResult, DatabaseError>(result)
            })
        })
        .join()
        .unwrap()
    }

    pub async fn get_recent_block_pointers(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        self.db
            .read()
            .await
            .db
            .load_recent_block_ptrs(number_of_blocks)
            .await
    }

    pub async fn commit_data(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        let time = Instant::now();
        let block_number = block_ptr.number;
        let mut db = self.db.write().await;
        db.migrate_from_mem_to_db(block_ptr).await?;
        info!(
            Database,
            "committed to database";
            block_number => block_number,
            exec_time => format!("{:?}", time.elapsed())
        );
        Ok(())
    }

    pub async fn flush_cache(&self) -> Result<(), DatabaseError> {
        self.db.write().await.mem.clear();
        info!(Database, "flushed entity cache");
        Ok(())
    }

    pub async fn revert_from_block(&self, block_number: u64) -> Result<(), DatabaseError> {
        warn!(Database, "Reverting data (probably due to reorg)"; revert_from_block_number => block_number);
        self.db
            .write()
            .await
            .revert_from_block(block_number)
            .await?;
        warn!(Database, "Database reverted OK"; revert_from_block_number => block_number);
        Ok(())
    }

    pub async fn remove_outdated_snapshots(&self, at_block: u64) -> Result<usize, DatabaseError> {
        let db = self.db.write().await;
        let entities = db.mem.get_latest_entity_ids();
        let count = db.db.remove_snapshots(entities, at_block).await?;
        info!(Database, "entities' snapshot removed"; number_of_entity => count);
        Ok(count)
    }

    pub async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        let mut db = self.db.write().await;

        if db.earliest_block < to_block {
            let removed = db.db.clean_data_history(to_block).await?;
            info!(
                Database,
                "cleaned up data history in database";
                to_block => to_block,
                removed => format!("{removed} records")
            );
            db.earliest_block = to_block;
            return Ok(removed);
        }

        Ok(0)
    }

    #[cfg(test)]
    pub fn empty(registry: &Registry) -> Self {
        let mem = MemoryDb::default();
        let db = ExternDB::None;
        let metrics = DatabaseMetrics::new(registry);
        let database = Database {
            mem,
            db,
            metrics,
            schema: SchemaLookup::default(),
            earliest_block: 0,
        };
        DatabaseAgent::from(database)
    }
}
