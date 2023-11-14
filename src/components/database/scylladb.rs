use crate::common::BlockPtr;
use crate::error;
use crate::errors::DatabaseError;
use scylla::transport::session::Session;
use scylla::SessionBuilder;

use super::RawEntity;

fn json_to_hashmap(json: serde_json::Map<String, serde_json::Value>) -> Option<RawEntity> {
    todo!()
}

pub struct Scylladb {
    session: Session,
    keyspace: String,
}

impl Scylladb {
    pub async fn new(uri: &str, keyspace: &str) -> Result<Self, DatabaseError> {
        let session: Session = SessionBuilder::new().known_node(uri).build().await?;
        let this = Self {
            session,
            keyspace: keyspace.to_owned(),
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
WHERE block_ptr_number = ? AND block_ptr_hash = ? AND id = ?
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
                return Ok(json_to_hashmap(values));
            } else {
                error!(Scylladb, "Not an json object"; data => json);
                return Err(DatabaseError::Invalid);
            }
        }

        Ok(None)
    }
}
