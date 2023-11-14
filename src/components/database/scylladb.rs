use super::schema_lookup::SchemaLookup;
use super::RawEntity;
use crate::common::BlockPtr;
use crate::error;
use crate::errors::DatabaseError;
use scylla::transport::session::Session;
use scylla::SessionBuilder;

pub struct Scylladb {
    session: Session,
    keyspace: String,
    schema_lookup: SchemaLookup,
}

impl Scylladb {
    pub async fn new(uri: &str, keyspace: &str) -> Result<Self, DatabaseError> {
        let session: Session = SessionBuilder::new().known_node(uri).build().await?;
        let this = Self {
            session,
            keyspace: keyspace.to_owned(),
            schema_lookup: SchemaLookup::default(),
        };
        Ok(this)
    }

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let raw_query = format!(
            r#"
SELECT JSON * from {}.{}
WHERE block_ptr_number = ? AND block_ptr_hash = ? AND id = ? AND is_deleted IS NULL
LIMIT 1
"#,
            self.keyspace, entity_type
        );
        let result = self
            .session
            .query(
                raw_query,
                (block_ptr.number as i64, block_ptr.hash, entity_id),
            )
            .await?;

        if let Ok(data) = result.single_row() {
            let json_row = data.columns.first().cloned().unwrap().unwrap();
            let json_row_as_str = json_row.into_string().unwrap();
            let json: serde_json::Value = serde_json::from_str(&json_row_as_str).unwrap();
            if let serde_json::Value::Object(values) = json {
                let result = self.schema_lookup.json_to_entity(entity_type, values);
                return Ok(Some(result));
            } else {
                error!(Scylladb, "Not an json object"; data => json);
                return Err(DatabaseError::Invalid);
            }
        }

        Ok(None)
    }

    async fn load_entity_latest(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let raw_query = format!(
            r#"
SELECT JSON * from {}.{}
WHERE id = ? AND is_deleted IS NULL
ORDER BY block_ptr_number DESC
LIMIT 1
"#,
            self.keyspace, entity_type
        );
        let result = self.session.query(raw_query, vec![entity_id]).await?;

        if let Ok(data) = result.single_row() {
            let json_row = data.columns.first().cloned().unwrap().unwrap();
            let json_row_as_str = json_row.into_string().unwrap();
            let json: serde_json::Value = serde_json::from_str(&json_row_as_str).unwrap();
            if let serde_json::Value::Object(values) = json {
                let result = self.schema_lookup.json_to_entity(entity_type, values);
                return Ok(Some(result));
            } else {
                error!(Scylladb, "Not an json object"; data => json);
                return Err(DatabaseError::Invalid);
            }
        }

        Ok(None)
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: String,
        data: RawEntity,
    ) -> Result<(), DatabaseError> {
        assert!(data.contains_key("id"));

        let mut json_data = self.schema_lookup.entity_to_json(&entity_type, data);

        json_data.insert(
            "block_ptr_number".to_string(),
            serde_json::Value::from(block_ptr.number),
        );

        json_data.insert(
            "block_ptr_hash".to_string(),
            serde_json::Value::from(block_ptr.hash),
        );

        json_data.insert("is_deleted".to_string(), serde_json::Value::Null);

        let json_data = serde_json::Value::Object(json_data);

        let query = format!(
            r#"
INSERT INTO {}.{} JSON ?
"#,
            self.keyspace, entity_type
        );

        self.session
            .query(query, vec![json_data.to_string()])
            .await?;

        Ok(())
    }
}
