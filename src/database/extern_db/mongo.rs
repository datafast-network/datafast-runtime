use super::ExternDBTrait;
use crate::common::BlockPtr;
use crate::errors::DatabaseError;
use crate::messages::EntityID;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Bytes;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;
use crate::schema_lookup::FieldKind;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::Binary;
use mongodb::bson::Bson;
use mongodb::bson::Document;
use mongodb::options::FindOneOptions;
use mongodb::options::FindOptions;
use mongodb::Client;
use mongodb::Collection;
use mongodb::Database;
use mongodb::IndexModel;
use std::collections::HashMap;
use std::str::FromStr;

impl From<Value> for Bson {
    fn from(value: Value) -> Self {
        match value {
            Value::String(str) => Bson::String(str),
            Value::Int(int) => Bson::Int32(int),
            Value::Int8(int8) => Bson::Int64(int8),
            Value::BigDecimal(decimal) => Bson::String(decimal.to_string()),
            Value::Bool(bool) => Bson::Boolean(bool),
            Value::List(list) => Bson::Array(list.into_iter().map(Bson::from).collect()),
            Value::Bytes(bytes) => Bson::Binary(Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: bytes.to_vec(),
            }),
            Value::BigInt(n) => Bson::String(n.to_string()),
            Value::Null => Bson::Null,
        }
    }
}

pub struct MongoDB {
    #[allow(dead_code)]
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

        this.create_entity_tables().await?;
        this.create_block_ptr_table().await?;
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

    fn bson_to_store_value(value: Bson, field_kind: &FieldKind) -> Value {
        match field_kind.kind {
            StoreValueKind::String => Value::String(value.as_str().unwrap().to_owned()),
            StoreValueKind::Int => Value::Int(value.as_i32().unwrap()),
            StoreValueKind::Int8 => Value::Int8(value.as_i64().unwrap()),
            StoreValueKind::Bool => Value::Bool(value.as_bool().unwrap()),
            StoreValueKind::Null => Value::Null,
            StoreValueKind::BigInt => {
                Value::BigInt(BigInt::from_str(value.as_str().unwrap()).unwrap())
            }
            StoreValueKind::BigDecimal => {
                Value::BigDecimal(BigDecimal::from_str(value.as_str().unwrap()).unwrap())
            }
            StoreValueKind::Bytes => {
                let bytes = Binary::from_base64(
                    value.as_str().unwrap(),
                    Some(mongodb::bson::spec::BinarySubtype::Generic),
                )
                .expect("failed to deserialize binary from bson");
                Value::Bytes(Bytes::from(bytes.bytes))
            }
            StoreValueKind::Array => {
                let values = value.as_array().cloned().unwrap();
                let inner_kind = field_kind.list_inner_kind.unwrap();
                let kind = FieldKind {
                    kind: inner_kind,
                    relation: None,
                    list_inner_kind: None,
                };
                let values = values
                    .into_iter()
                    .map(|inner_val| Self::bson_to_store_value(inner_val, &kind))
                    .collect();
                Value::List(values)
            }
        }
    }

    fn document_to_raw_entity(
        schema_lookup: &SchemaLookup,
        entity_type: &str,
        doc: Document,
    ) -> RawEntity {
        let mut result = RawEntity::new();

        for (field_name, value) in doc {
            let field_kind = schema_lookup.get_field(entity_type, &field_name);
            result.insert(field_name, Self::bson_to_store_value(value, &field_kind));
        }
        result
    }
}

#[async_trait]
impl ExternDBTrait for MongoDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        for (_, collection) in self.entity_collections.iter() {
            let idx_model = IndexModel::builder()
                .keys(doc! { "id": 1, "__block_ptr__": -1 })
                .build();
            collection.create_index(idx_model, None).await?;
        }
        Ok(())
    }

    async fn create_block_ptr_table(&self) -> Result<(), DatabaseError> {
        let idx_model = IndexModel::builder()
            .keys(doc! { "block_number": -1 })
            .build();
        self.block_ptr_collection
            .create_index(idx_model, None)
            .await?;
        Ok(())
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
            .map(|doc| Self::document_to_raw_entity(&self.schema, entity_type, doc));
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
            .map(|doc| Self::document_to_raw_entity(&self.schema, entity_type, doc));
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

    async fn load_recent_block_ptrs(
        &self,
        number_of_blocks: u16,
    ) -> Result<Vec<BlockPtr>, DatabaseError> {
        let options = FindOptions::builder()
            .sort(doc! {"block_number": -1})
            .limit(number_of_blocks as i64)
            .build();
        let cursor = self.block_ptr_collection.find(None, options).await?;
        let result = cursor
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .flatten()
            .collect();
        Ok(result)
    }

    async fn get_earliest_block_ptr(&self) -> Result<Option<BlockPtr>, DatabaseError> {
        let opts = FindOneOptions::builder()
            .sort(doc! { "block_ptr": 1 })
            .build();
        self.block_ptr_collection
            .find_one(None, Some(opts))
            .await
            .map_err(DatabaseError::from)
    }

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        self.block_ptr_collection
            .insert_one(block_ptr, None)
            .await?;
        Ok(())
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        let collection = self
            .entity_collections
            .get(entity_type)
            .expect("Entity not exists!");
        let cursor = collection.find(doc! { "id": { "$in": ids }}, None).await?;
        let result = cursor
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .flatten()
            .map(|doc| Self::document_to_raw_entity(&self.schema, entity_type, doc))
            .collect();
        Ok(result)
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(EntityType, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        todo!()
    }

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError> {
        todo!()
    }

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;
    use crate::schema_lookup::Schema;
    use std::env;

    async fn setup(entity_type: &str) -> Result<MongoDB, DatabaseError> {
        env_logger::try_init().unwrap_or_default();
        let uri = env::var("MONGO_URI").unwrap();
        let database_name = env::var("MONGO_DATABASE").unwrap();
        let mut schema = SchemaLookup::new();

        let mut test_schema: Schema = schema!(
            id => StoreValueKind::String,
            name => StoreValueKind::String,
            symbol => StoreValueKind::String,
            total_supply => StoreValueKind::BigInt,
            userBalance => StoreValueKind::BigInt,
            tokenBlockNumber => StoreValueKind::BigInt,
            users => StoreValueKind::Array,
            table => StoreValueKind::String
        );

        test_schema.get_mut("users").unwrap().list_inner_kind = Some(StoreValueKind::String);

        schema.add_schema(entity_type, test_schema);
        let db = MongoDB::new(&uri, &database_name, schema).await?;
        db.drop_db().await?;
        Ok(db)
    }

    #[tokio::test]
    async fn test_01_init() {
        let db = setup("token_01").await.unwrap();
    }
}
