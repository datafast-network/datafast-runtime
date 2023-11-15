use super::RawEntity;
use crate::error;
use crate::errors::DatabaseError;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::runtime::asc::native_types::store::Value;
use std::collections::HashMap;

type EntityPayload = HashMap<String, Value>;
type EntitySnapshots = Vec<EntityPayload>;

#[derive(Default, Debug)]
pub struct MemoryDb(HashMap<EntityType, HashMap<EntityID, EntitySnapshots>>);

impl MemoryDb {
    pub fn load_entity_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let store = &self.0;
        let table = store.get(&entity_type);

        if table.is_none() {
            return Ok(None);
        }

        let table = table.unwrap();
        let entity = table.get(&entity_id);

        if entity.is_none() {
            return Ok(None);
        }

        let data = entity.unwrap().last().cloned().unwrap();
        let is_deleted = data.get("is_deleted").cloned().unwrap();
        if let Value::Bool(is_deleted) = is_deleted {
            if is_deleted {
                return Ok(None);
            }
        } else {
            error!(MemoryDb, "is_deleted is not a bool";
                entity_type => entity_type.clone(),
                entity_id => entity_id.clone()
            );
            unimplemented!("is_deleted is not a bool")
        }
        Ok(Some(data))
    }

    pub fn create_entity(
        &mut self,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        let store = &mut self.0;
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
            let mut new_data = data.clone();
            new_data.insert("is_deleted".to_string(), Value::Bool(false));
            snapshots.push(new_data);
            Ok(())
        } else {
            Err(DatabaseError::InvalidValue("id".to_string()))
        }
    }

    pub fn soft_delete(
        &mut self,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), DatabaseError> {
        let store = &mut self.0;
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
            snapshot.insert("is_deleted".to_string(), Value::Bool(true));
        }

        Ok(())
    }

    pub fn hard_delete(
        &mut self,
        entity_type: String,
        entity_id: String,
    ) -> Result<(), DatabaseError> {
        let store = &mut self.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_memory_01_db_insert() {
        env_logger::try_init().unwrap_or_default();
        let mut db = MemoryDb::default();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("1".to_string()));
        data.insert("name".to_string(), Value::String("test".to_string()));

        let result = db.create_entity("test".to_string(), data);
        assert!(result.is_ok());
        let latest = db.load_entity_latest("test".to_string(), "1".to_string());
        assert!(latest.is_ok());
        let latest = latest.unwrap();
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(
            latest.get("name").unwrap(),
            &Value::String("test".to_string())
        );
        assert_eq!(latest.get("id").unwrap(), &Value::String("1".to_string()));
        assert_eq!(latest.get("is_deleted").unwrap(), &Value::Bool(false));
    }

    #[test]
    fn test_memory_02_db_delete() {
        env_logger::try_init().unwrap_or_default();
        let mut db = MemoryDb::default();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("1".to_string()));
        data.insert("name".to_string(), Value::String("test".to_string()));

        let result = db.create_entity("test".to_string(), data);
        assert!(result.is_ok());
        let latest = db.load_entity_latest("test".to_string(), "1".to_string());
        assert!(latest.is_ok());
        let latest = latest.unwrap();
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(
            latest.get("name").unwrap(),
            &Value::String("test".to_string())
        );
        assert_eq!(latest.get("id").unwrap(), &Value::String("1".to_string()));
        assert_eq!(latest.get("is_deleted").unwrap(), &Value::Bool(false));

        let result = db.soft_delete("test".to_string(), "1".to_string());
        assert!(result.is_ok());
        let latest = db
            .load_entity_latest("test".to_string(), "1".to_string())
            .unwrap();
        assert!(latest.is_none());
    }
}
