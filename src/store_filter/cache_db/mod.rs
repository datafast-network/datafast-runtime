mod redis;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::errors::CacheError;


#[async_trait]
pub trait CacheTrait {
    async fn get(&self, key: &str) -> Result<Vec<u8>, CacheError>;
    async fn set<T: DeserializeOwned>(&self, key: &str, value: T) -> Result<(), CacheError> where T: Serialize + Send;
    async fn del(&self, key: &str) -> Result<(), CacheError>;
}

#[derive(Clone)]
pub enum CacheDb {
    Redis(redis::RedisCache)
}

#[async_trait]
impl CacheTrait for CacheDb {
    async fn get(&self, key: &str) -> Result<Vec<u8>, CacheError> {
        match self {
            CacheDb::Redis(cache) => cache.get(key).await
        }
    }

    async fn set<T: DeserializeOwned>(&self, key: &str, value: T) -> Result<(), CacheError> where T: Serialize + Send {
        match self {
            CacheDb::Redis(cache) => cache.set(key, value).await
        }
    }

    async fn del(&self, key: &str) -> Result<(), CacheError> {
        match self {
            CacheDb::Redis(cache) => cache.del(key).await
        }
    }
}

pub async fn create_redis_cache(uri: String) -> Result<CacheDb, CacheError> {
    let cache = redis::RedisCache::new(uri).await?;
    Ok(CacheDb::Redis(cache))
}

