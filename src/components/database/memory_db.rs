use crate::error;
use crate::errors::DatabaseError;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Value;
use std::collections::HashMap;

type EntitySnapshots = Vec<RawEntity>;

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
        let is_deleted = data
            .get("is_deleted")
            .cloned()
            .ok_or(DatabaseError::MissingField("is_deleted".to_string()))?;
        if let Value::Bool(is_deleted) = is_deleted {
            if is_deleted {
                return Ok(None);
            }
        } else {
            error!(MemoryDb, "is_deleted is not a bool";
                entity_type => entity_type,
                entity_id => entity_id
            );
            return Err(DatabaseError::InvalidValue("is_deleted".to_string()));
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
            error!(MemoryDb, "id is invalid";
                entity_type => entity_type,
                rawEntity => format!("{:?}", data)
            );
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
        let mut last = snapshots.iter().last().cloned().unwrap();
        last.remove("is_deleted");
        last.insert("is_deleted".to_string(), Value::Bool(true));
        snapshots.push(last);

        Ok(())
    }

    pub fn extract_data(&self) -> Result<Vec<(String, RawEntity)>, DatabaseError> {
        let mut result = vec![];
        self.0.iter().for_each(|(entity_type, table)| {
            table.iter().for_each(|(_entity_id, snapshots)| {
                if let Some(last) = snapshots.last().cloned() {
                    result.push((entity_type.clone(), last));
                }
            });
        });

        Ok(result)
    }

    pub fn clear(&mut self) {
        self.0.clear()
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

        db.create_entity("test".to_string(), data).unwrap();
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

        db.soft_delete("test".to_string(), "1".to_string()).unwrap();

        let latest = db
            .load_entity_latest("test".to_string(), "1".to_string())
            .unwrap();
        assert!(latest.is_none());
        assert_eq!(db.0.get("test").unwrap().get("1").unwrap().len(), 2);
    }
    #[test]
    fn test_memory_03_extract_data() {
        env_logger::try_init().unwrap_or_default();
        let mut db = MemoryDb::default();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("1".to_string()));
        data.insert("name".to_string(), Value::String("test".to_string()));
        db.create_entity("test".to_string(), data).unwrap();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("1".to_string()));
        data.insert("name".to_string(), Value::String("test111".to_string()));
        db.create_entity("test".to_string(), data).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("2".to_string()));
        data.insert("name".to_string(), Value::String("test22".to_string()));
        db.create_entity("test2".to_string(), data).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("2".to_string()));
        data.insert("name".to_string(), Value::String("test222".to_string()));
        db.create_entity("test2".to_string(), data).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("3".to_string()));
        data.insert("name".to_string(), Value::String("test".to_string()));
        db.create_entity("test2".to_string(), data).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("3".to_string()));
        data.insert("name".to_string(), Value::String("test333".to_string()));
        db.create_entity("test2".to_string(), data).unwrap();

        let extract_data = db.extract_data().unwrap();
        log::info!("extract_data: {:?}", extract_data);
        assert_eq!(extract_data.len(), 3);
    }
}
