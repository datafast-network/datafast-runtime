use super::DatabaseTrait;
use super::RawEntity;
use crate::common::BlockPtr;
use crate::errors::DatabaseError;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::runtime::asc::native_types::store::Value;
use std::collections::HashMap;

type BlockPtrNumber = u64;
type BlockPtrHash = String;
type DeletedAt = Option<u64>;
type EntityPayload = HashMap<String, Value>;
type EntitySnapshots = Vec<(BlockPtrNumber, BlockPtrHash, DeletedAt, EntityPayload)>;

pub type InMemoryDataStore = HashMap<EntityType, HashMap<EntityID, EntitySnapshots>>;

impl DatabaseTrait for InMemoryDataStore {
    fn handle_load(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let store = self;
        let table = store.get(&entity_type);

        if table.is_none() {
            return Ok(None);
        }

        let table = table.unwrap();
        let entity = table.get(&entity_id);

        if entity.is_none() {
            return Ok(None);
        }

        for row in entity.unwrap() {
            if row.0 == block_ptr.number && row.1 == block_ptr.hash {
                return Ok(Some(row.3.to_owned()));
            }
        }

        return Ok(None);
    }

    fn handle_load_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let store = self;
        let table = store.get(&entity_type);

        if table.is_none() {
            return Ok(None);
        }

        let table = table.unwrap();
        let entity = table.get(&entity_id);

        if entity.is_none() {
            return Ok(None);
        }

        let (_, _, _, data) = entity.unwrap().last().cloned().unwrap();
        return Ok(Some(data));
    }

    fn handle_create(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        let store = self;
        if !store.contains_key(&entity_type) {
            store.insert(entity_type.clone(), HashMap::new());
        }

        let table = store.get_mut(&entity_type).unwrap();
        if let Value::String(entity_id) = data.get("id").ok_or(DatabaseError::MissingID)? {
            // Check if this id exists or not
            if table.get(entity_id).is_none() {
                table.insert(entity_id.to_owned(), vec![]);
            };

            // Push new record
            let snapshots = table.get_mut(entity_id).unwrap();
            snapshots.push((block_ptr.number, block_ptr.hash, None, data));

            Ok(())
        } else {
            Err(DatabaseError::InvalidValue("id".to_string()))
        }
    }

    fn soft_delete(
        &mut self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), DatabaseError> {
        let store = self;
        let table = store.get_mut(&entity_type);

        if table.is_none() {
            return Err(DatabaseError::EntityTypeNotExists(entity_type));
        }

        let table = table.unwrap();
        let entity = table.get_mut(&entity_id);

        if entity.is_none() {
            return Err(DatabaseError::EntityIDNotExists(entity_type, entity_id));
        }

        let snapshots = entity.unwrap();
        for snapshot in snapshots.iter_mut() {
            snapshot.2 = Some(block_ptr.number);
        }

        Ok(())
    }

    fn hard_delete(&mut self, entity_type: String, entity_id: String) -> Result<(), DatabaseError> {
        let store = self;
        let table = store.get_mut(&entity_type);

        if table.is_none() {
            return Err(DatabaseError::EntityTypeNotExists(entity_type));
        }

        let table = table.unwrap();
        let entity = table.get_mut(&entity_id);

        if entity.is_none() {
            return Err(DatabaseError::EntityIDNotExists(entity_type, entity_id));
        }

        table.remove_entry(&entity_id);

        Ok(())
    }
}
