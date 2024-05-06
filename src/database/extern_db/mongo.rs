use super::ExternDBTrait;
use crate::common::BlockPtr;
use crate::common::Datasource;
use crate::common::EntityID;
use crate::common::EntityType;
use crate::common::FieldKind;
use crate::common::RawEntity;
use crate::common::Schemas;
use crate::errors::DatabaseError;
use df_types::asc::native_types::store::Bytes;
use df_types::asc::native_types::store::StoreValueKind;
use df_types::asc::native_types::store::Value;
use df_types::bignumber::bigdecimal::BigDecimal;
use df_types::bignumber::bigint::BigInt;
use async_trait::async_trait;
use df_logger::info;
use futures_util::future::try_join_all;
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::Binary;
use mongodb::bson::Bson;
use mongodb::bson::Document;
use mongodb::options::DatabaseOptions;
use mongodb::options::FindOneOptions;
use mongodb::options::FindOptions;
use mongodb::options::IndexOptions;
use mongodb::options::InsertManyOptions;
use mongodb::options::WriteConcern;
use mongodb::Client;
use mongodb::Collection;
use mongodb::Database;
use mongodb::IndexModel;
use serde::Deserialize;
use serde::Serialize;
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct WrappedDatasource {
    pub name: String,
    pub address: Option<String>,
    pub created_at_block: Option<u64>,
    pub datasource: Datasource,
}

impl From<Datasource> for WrappedDatasource {
    fn from(ds: Datasource) -> Self {
        let name = ds.name.clone();
        let address = ds.source.address.clone();
        let created_at_block = ds.source.startBlock;
        WrappedDatasource {
            datasource: ds,
            name,
            address,
            created_at_block,
        }
    }
}

pub struct MongoDB {
    #[allow(dead_code)]
    db: Database,
    schemas: Schemas,
    entity_collections: HashMap<EntityType, Collection<Document>>,
    block_ptr_collection: Collection<BlockPtr>,
    datasource_collection: Collection<WrappedDatasource>,
}

impl MongoDB {
    pub async fn new(
        uri: &str,
        database_name: &str,
        schemas: Schemas,
    ) -> Result<Self, DatabaseError> {
        let client = Client::with_uri_str(uri).await?;
        info!(Database, "client created OK");
        let db_options = DatabaseOptions::builder()
            .write_concern(Some(WriteConcern::MAJORITY))
            .build();
        let db = client.database_with_options(database_name, db_options);
        info!(Database, "db namespace created OK");

        let block_ptr_collection = db.collection::<BlockPtr>("block_ptr");
        let entity_collections = schemas
            .get_entity_names()
            .into_iter()
            .map(|entity_type| {
                let collection = db.collection::<Document>(&entity_type);
                (entity_type.to_owned(), collection)
            })
            .collect::<HashMap<EntityType, Collection<Document>>>();
        let datasource_collection = db.collection::<WrappedDatasource>("datasources");

        let this = MongoDB {
            db,
            schemas,
            entity_collections,
            block_ptr_collection,
            datasource_collection,
        };

        this.create_entity_tables().await?;
        info!(Database, "entity-tables created OK");
        this.create_block_ptr_table().await?;
        info!(Database, "block-ptr created OK");
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
                if let Bson::Binary(bytes) = value {
                    Value::Bytes(Bytes::from(bytes.bytes))
                } else {
                    panic!()
                }
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

    fn document_to_raw_entity(schemas: &Schemas, entity_type: &str, doc: Document) -> RawEntity {
        let mut result = RawEntity::new();

        for (field_name, value) in doc {
            let field_kind = schemas.get_field(entity_type, &field_name);
            result.insert(field_name, Self::bson_to_store_value(value, &field_kind));
        }
        result
    }
}

#[async_trait]
impl ExternDBTrait for MongoDB {
    async fn create_entity_tables(&self) -> Result<(), DatabaseError> {
        let idx_option = IndexOptions::builder().unique(true).build();
        for (_, collection) in self.entity_collections.iter() {
            let idx_model = IndexModel::builder()
                .keys(doc! { "id": -1, "__block_ptr__": -1 })
                .options(idx_option.clone())
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

    async fn create_datasource_table(&self) -> Result<(), DatabaseError> {
        let opts = IndexOptions::builder().unique(true).build();
        let idx_model = IndexModel::builder()
            .keys(doc! { "created_at_block": -1, "name": 1, "address": 1 })
            .options(opts)
            .build();
        self.datasource_collection
            .create_index(idx_model, None)
            .await?;
        Ok(())
    }

    async fn load_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<RawEntity>, DatabaseError> {
        let collection = self
            .entity_collections
            .get(entity_type)
            .expect("Entity not exists!");
        let filter = doc! { "id": entity_id };
        let opts = FindOneOptions::builder()
            .sort(doc! { "block_ptr": -1 })
            .projection(doc! { "_id": 0 })
            .build();
        let result = collection
            .find_one(filter, Some(opts))
            .await?
            .map(|doc| Self::document_to_raw_entity(&self.schemas, entity_type, doc));

        if result.is_none() {
            return Ok(None);
        }

        let entity = result.unwrap();

        let is_deleted = entity.get("__is_deleted__").cloned().unwrap();

        if is_deleted == Value::Bool(true) {
            return Ok(None);
        }

        Ok(Some(entity))
    }

    async fn load_entities(
        &self,
        entity_type: &str,
        ids: Vec<String>,
    ) -> Result<Vec<RawEntity>, DatabaseError> {
        let fetch_entities = ids
            .iter()
            .map(|entity_id| self.load_entity(entity_type, entity_id));
        let result = try_join_all(fetch_entities)
            .await?
            .into_iter()
            .flatten()
            .collect();
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

    async fn save_block_ptr(&self, block_ptr: BlockPtr) -> Result<(), DatabaseError> {
        self.block_ptr_collection
            .insert_one(block_ptr, None)
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

    async fn save_datasources(&self, datasources: Vec<Datasource>) -> Result<(), DatabaseError> {
        let docs: Vec<_> = datasources
            .into_iter()
            .map(WrappedDatasource::from)
            .collect();

        // Allow duplicate error, and just insert other datasources
        let opts = InsertManyOptions::builder().ordered(false).build();
        self.datasource_collection
            .insert_many(docs, opts)
            .await
            .ok();
        Ok(())
    }

    async fn load_datasources(&self) -> Result<Option<Vec<Datasource>>, DatabaseError> {
        let cursor = self.datasource_collection.find(doc! {}, None).await?;
        let result = cursor
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .flatten()
            .map(|wds| wds.datasource)
            .collect::<Vec<_>>();

        if result.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))
    }

    async fn batch_insert_entities(
        &self,
        block_ptr: BlockPtr,
        values: Vec<(EntityType, RawEntity)>,
    ) -> Result<(), DatabaseError> {
        let mut grouped_values = HashMap::<EntityType, Vec<RawEntity>>::new();

        for (entity_type, mut data) in values {
            grouped_values
                .entry(entity_type.to_owned())
                .or_insert_with(std::vec::Vec::new);

            data.remove("__block_ptr__");
            data.insert(
                "__block_ptr__".to_string(),
                Value::Int8(block_ptr.number as i64),
            );

            grouped_values.get_mut(&entity_type).unwrap().push(data);
        }

        let mut inserts = vec![];
        for (entity_type, records) in grouped_values {
            let collection = self
                .entity_collections
                .get(&entity_type)
                .expect("Entity type not exists!");
            let docs = records
                .into_iter()
                .map(Self::raw_entity_to_document)
                .collect::<Vec<Document>>();
            inserts.push(collection.insert_many(docs.clone(), None));
        }

        let result = try_join_all(inserts).await?;
        info!(
            Database,
            "Commit result";
            statements => format!("{:?} statements", result.len())
        );
        Ok(())
    }

    async fn revert_from_block(&self, from_block: u64) -> Result<(), DatabaseError> {
        let mut tasks = vec![];
        for c in self.entity_collections.values() {
            tasks.push(c.delete_many(
                doc! { "__block_ptr__": { "$gte": from_block as i64 } },
                None,
            ));
        }
        try_join_all(tasks).await?;
        Ok(())
    }

    async fn remove_snapshots(
        &self,
        entities: Vec<(EntityType, EntityID)>,
        to_block: u64,
    ) -> Result<usize, DatabaseError> {
        let mut tasks = vec![];
        for (entity_type, entity_id) in entities {
            let c = self.entity_collections.get(&entity_type).unwrap();
            tasks.push(c.delete_many(
                doc! { "__block_ptr__": { "$lt": to_block as i64 }, "id": entity_id },
                None,
            ));
        }
        try_join_all(tasks).await?;
        Ok(0)
    }

    async fn clean_data_history(&self, to_block: u64) -> Result<u64, DatabaseError> {
        let mut tasks = vec![];
        for c in self.entity_collections.values() {
            tasks.push(c.delete_many(doc! { "__block_ptr__": { "$lt": to_block as i64 } }, None));
        }
        try_join_all(tasks).await?;
        Ok(1)
    }

    fn get_schema(&self) -> Schemas {
        self.schemas.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Schema;
    use crate::entity;
    use crate::schema;
    use df_logger::log;
    use df_logger::loggers::init_logger;
    use futures_util::StreamExt;
    use std::env;
    use std::time::Instant;

    async fn setup(entity_type: &str) -> Result<(MongoDB, String), DatabaseError> {
        init_logger();
        let uri =
            env::var("MONGO_URI").unwrap_or("mongodb://root:example@localhost:27017".to_string());
        let database_name = env::var("MONGO_DATABASE").unwrap_or("db0".to_string());
        MongoDB::new(&uri, &database_name, Schemas::default())
            .await?
            .drop_db()
            .await?;

        let mut schema = Schemas::default();

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

        schema.add_schema(entity_type, test_schema, None);

        let test_schema_2: Schema = schema!(
            id => StoreValueKind::String,
            data => StoreValueKind::Bytes
        );

        schema.add_schema("entity_with_data", test_schema_2, None);

        let db = MongoDB::new(&uri, &database_name, schema).await?;
        Ok((db, entity_type.to_owned()))
    }

    #[tokio::test]
    async fn test_data() {
        let (db, _) = setup("token_00").await.unwrap();
        let example_byte = "0x000000000000000000000000000000000000000000000000000000000c119fea";
        let item: RawEntity = entity! {
            id => Value::String("item".to_string()),
            data => Value::Bytes(Bytes::from(hex::decode(example_byte.replace("0x", "")).unwrap())),
            __is_deleted__ => Value::Bool(false)
        };
        db.create_entity(BlockPtr::default(), "entity_with_data", item.clone())
            .await
            .unwrap();

        let loaded = db
            .load_entity("entity_with_data", "item")
            .await
            .unwrap()
            .unwrap();
        let returned_data = loaded.get("data").cloned().unwrap();
        if let Value::Bytes(bytes) = returned_data {
            let actual_data = format!("0x{}", hex::encode(bytes.to_vec()));
            assert_eq!(actual_data, example_byte);
        } else {
            panic!()
        }
    }

    #[tokio::test]
    async fn test_01() {
        let (db, entity_type) = setup("token_01").await.unwrap();

        let tk1: RawEntity = entity! {
            id => Value::String("token-id".to_string()),
            name => Value::String("Tether USD".to_string()),
            symbol => Value::String("USDT".to_string()),
            total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap()),
            userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
            tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
            users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
            table => Value::String("dont-matter".to_string()),
            __is_deleted__ => Value::Bool(false)
        };
        db.create_entity(BlockPtr::default(), &entity_type, tk1)
            .await
            .unwrap();
        let loaded = db.load_entity(&entity_type, "token-id").await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(
            loaded.get("__is_deleted__").cloned().unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            loaded.get("__block_ptr__").cloned().unwrap(),
            Value::Int8(0)
        );
        assert_eq!(
            loaded.get("id").cloned().unwrap(),
            Value::String("token-id".to_string())
        );

        let tk2: RawEntity = entity! {
            id => Value::String("token-id".to_string()),
            name => Value::String("Tether USD".to_string()),
            symbol => Value::String("USDT".to_string()),
            total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap()),
            userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
            tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
            users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
            table => Value::String("dont-matter".to_string()),
            __is_deleted__ => Value::Bool(true)
        };
        let duplicate_insert = db
            .create_entity(BlockPtr::default(), &entity_type, tk2)
            .await;
        assert!(duplicate_insert.is_err());

        let tk3: RawEntity = entity! {
            id => Value::String("token-id-1".to_string()),
            name => Value::String("Tether USD".to_string()),
            symbol => Value::String("USDT".to_string()),
            total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap()),
            userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
            tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
            users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
            table => Value::String("dont-matter".to_string()),
            __is_deleted__ => Value::Bool(true)
        };
        db.create_entity(BlockPtr::default(), &entity_type, tk3)
            .await
            .unwrap();
        let loaded = db.load_entity(&entity_type, "token-id-1").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_02() {
        let (db, entity_type) = setup("token_02").await.unwrap();

        for i in 1..10 {
            let block_ptr = BlockPtr {
                number: i,
                hash: format!("i={i}"),
                parent_hash: format!("i={}", i - 1),
            };

            let mut entities = vec![];

            for j in 0..1000 {
                let tk: RawEntity = entity! {
                    id => Value::String(format!("token_{j}")),
                    name => Value::String("Tether USD".to_string()),
                    symbol => Value::String("USDT".to_string()),
                    total_supply => Value::BigInt(BigInt::from_str("111222333444555666777888999").unwrap()),
                    userBalance => Value::BigInt(BigInt::from_str("10").unwrap()),
                    tokenBlockNumber => Value::BigInt(BigInt::from_str("100").unwrap()),
                    users => Value::List(vec![Value::String("vu".to_string()),Value::String("quan".to_string())]),
                    table => Value::String("dont-matter".to_string()),
                    __is_deleted__ => Value::Bool(false)
                };
                entities.push((entity_type.to_owned(), tk));
            }

            let timer = Instant::now();
            db.batch_insert_entities(block_ptr.clone(), entities)
                .await
                .unwrap();
            log::info!("Done batch insert in {:?}", timer.elapsed());
            db.save_block_ptr(block_ptr.clone()).await.unwrap();

            for token_number in 0..10 {
                let entity = db
                    .load_entity(&entity_type, &format!("token_{token_number}"))
                    .await
                    .unwrap();
                assert!(entity.is_some());
            }

            let token_ids = (0..10)
                .map(|i| format!("token_{i}"))
                .collect::<Vec<EntityID>>();
            let tokens = db.load_entities(&entity_type, token_ids).await.unwrap();
            assert_eq!(tokens.len(), 10);

            for token in tokens {
                assert_eq!(
                    token.get("__block_ptr__").cloned().unwrap(),
                    Value::Int8(block_ptr.number as i64)
                );
            }
        }

        log::info!("Testing revert.........");
        db.revert_from_block(9).await.unwrap();
        let token_ids = (0..10)
            .map(|i| format!("token_{i}"))
            .collect::<Vec<EntityID>>();
        let tokens = db.load_entities(&entity_type, token_ids).await.unwrap();
        assert_eq!(tokens.len(), 10);

        for token in tokens {
            assert_eq!(token.get("__block_ptr__").cloned().unwrap(), Value::Int8(8));
        }

        log::info!("Testing remove-snapshots.........");
        let collection = db.entity_collections.get(&entity_type).unwrap();
        let token1 = collection
            .find(doc! { "id": "token_1" }, None)
            .await
            .unwrap()
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(token1.len(), 8);
        db.remove_snapshots(vec![(entity_type.clone(), "token_1".to_string())], 8)
            .await
            .unwrap();
        let token1 = collection
            .find(doc! { "id": "token_1" }, None)
            .await
            .unwrap()
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(token1.len(), 1);
    }
}
