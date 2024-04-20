use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::database::cache_db::RedisCache;
use crate::errors::CacheError;

pub enum Cache {
    Redis(RedisCache),
}

#[async_trait]
pub trait ICache {
    async fn get<T>(&self, key: &str) -> Result<T, CacheError> where T: DeserializeOwned + Send;
    async fn set<T: DeserializeOwned>(&self, key: &str, value: T) -> Result<(), CacheError> where T: Serialize + Send;
    async fn del(&self, key: &str) -> Result<(), CacheError>;
}

impl ICache for Cache {
    async fn get<T>(&self, key: &str) -> Result<T, CacheError> where T: DeserializeOwned + Send {
        match self {
            Cache::Redis(cache) => cache.get(key).await
        }
    }

    async fn set<T: DeserializeOwned>(&self, key: &str, value: T) -> Result<(), CacheError> where T: Serialize + Send {
        match self {
            Cache::Redis(cache) => cache.set(key, value).await
        }
    }

    async fn del(&self, key: &str) -> Result<(), CacheError> {
        match self {
            Cache::Redis(cache) => cache.del(key).await
        }
    }
}

pub async fn create_redis_cache(uri: String) -> Result<Cache, CacheError> {
    let cache = RedisCache::new(uri).await?;
    Ok(Cache::Redis(cache))
}