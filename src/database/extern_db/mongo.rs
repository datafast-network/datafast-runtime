use super::ExternDBTrait;
use crate::common::BlockPtr;
use crate::errors::DatabaseError;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Value;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::bson::Bson;
use mongodb::bson::Document;
use mongodb::options::FindOneOptions;
use mongodb::Client;
use mongodb::Collection;
use mongodb::Database;
use std::collections::HashMap;

impl From<Value> for Bson {
    fn from(value: Value) -> Self {
        match value {
            Value::String(str) => Bson::String(str),
            Value::Int(int) => Bson::Int32(int),
            Value::Int8(int8) => Bson::Int64(int8),
            Value::BigDecimal(decimal) => Bson::String(decimal.to_string()),
            Value::Bool(bool) => Bson::Boolean(bool),
            Value::List(list) => Bson::Array(list.into_iter().map(Bson::from).collect()),
            Value::Bytes(bytes) => Bson::Binary(mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: bytes.to_vec(),
            }),
            Value::BigInt(n) => Bson::String(n.to_string()),
            Value::Null => Bson::Null,
        }
    }
}

pub struct MongoDB {
    db: Database,
    schema: SchemaLookup,
    entity_collections: HashMap<EntityType, Collection<Document>>,
    block_ptr_collection: Collection<BlockPtr>,
}

impl MongoDB {
    pub async fn new(
        uri: &str,
        database_name: &str,
        schema: SchemaLookup,
    ) -> Result<Self, DatabaseError> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database(database_name);

        let block_ptr_collection = db.collection::<BlockPtr>("block_ptr");
        let entity_collections = schema
            .get_entity_names()
            .into_iter()
            .map(|entity_type| {
                let collection = db.collection::<Document>(&entity_type);
                (entity_type.to_owned(), collection)
            })
            .collect::<HashMap<EntityType, Collection<Document>>>();

        let this = MongoDB {
            db,
            schema,
            entity_collections,
            block_ptr_collection,
        };
        Ok(this)
    }

    #[cfg(test)]
    async fn drop_db(&self) -> Result<(), DatabaseError> {
        use mongodb::options::DropDatabaseOptions;

        let opts = DropDatabaseOptions::builder().build();
        self.db.drop(opts).await?;
        Ok(())
    }

    fn raw_entity_to_document(entity: RawEntity) -> Document {
        let mut result = doc! {};

        for (field, value) in entity.iter() {
            result.insert(field.to_owned(), Into::<Bson>::into(value));
        }

        result
    }

    fn document_to_raw_entity(doc: Document) -> RawEntity {
        let mut result = RawEntity::new();

        for (field, value) in doc.iter() {
            // result.insert(field.to_owned(), Value::from(value, field, ));
            todo!()
        }

        result
    }
}

#[async_trait]
impl ExternDBTrait for MongoDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        unimplemented!()
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        unimplemented!()
    }

    async fn load_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let collection = self
            .entity_collections
            .get(entity_type)
            .expect("Entity not exists!");
        let filter = doc! {
            "__block_ptr__": block_ptr.number as i64,
            "id": entity_id,
        };
        let result = collection
            .find_one(filter, None)
            .await?
            .map(Self::document_to_raw_entity);
        Ok(result)
    }

    async fn load_entity_latest(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let collection = self
            .entity_collections
            .get(entity_type)
            .expect("Entity not exists!");
        let filter = doc! { "id": entity_id, "__is_deleted__": false };
        let opts = FindOneOptions::builder()
            .sort(doc! { "block_ptr": -1 })
            .build();
        let result = collection
            .find_one(filter, Some(opts))
            .await?
            .map(Self::document_to_raw_entity);
        Ok(result)
    }

    async fn create_entity(
        &self,
        block_ptr: BlockPtr,
        entity_type: &str,
        mut data: RawEntity,
    ) -> Result<(), DatabaseError> {
        let collection = self
            .entity_collections
            .get(entity_type)
            .expect("Entity not exists!");

        data.remove("__block_ptr__");
        data.insert(
            "__block_ptr__".to_string(),
            Value::Int8(block_ptr.number as i64),
        );
        collection
            .insert_one(Self::raw_entity_to_document(data), None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MongoDB;
    use crate::{errors::DatabaseError, schema_lookup::SchemaLookup};
    use std::env;

    async fn setup() -> Result<MongoDB, DatabaseError> {
        env_logger::try_init().unwrap_or_default();
        let uri = env::var("MONGO_URI").unwrap();
        let database_name = env::var("MONGO_DATABASE").unwrap();
        let schema = SchemaLookup::new();
        let db = MongoDB::new(&uri, &database_name, schema).await?;
        db.drop_db().await?;
        Ok(db)
    }

    #[tokio::test]
    async fn test_01_init() {
        let db = setup().await;
    }
}
