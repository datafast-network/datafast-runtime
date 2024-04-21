use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use redis::aio::MultiplexedConnection;
use serde::de::DeserializeOwned;
use serde::{Serialize};
use crate::errors::CacheError;
use crate::store_filter::cache_db::CacheTrait;

#[derive(Clone)]
pub struct RedisCache {
    conn: MultiplexedConnection,
}

impl RedisCache {
    pub async fn new(uri: String) -> Result<Self, CacheError> {
        let client = Client::open(uri)
            .map_err(|e| CacheError::Initialization(e.to_string()))?;
        let conn = client.get_multiplexed_tokio_connection()
            .await
            .map_err(|e| CacheError::Initialization(e.to_string()))?;
        Ok(
            RedisCache {
                conn,
            }
        )
    }
}


#[async_trait]
impl CacheTrait for RedisCache {
    async fn get(&self, key: &str) -> Result<Vec<u8>, CacheError> {
        let mut conn = self.conn.clone();
        conn.get(key).await.map_err(CacheError::RedisError)
    }

    async fn set<T: DeserializeOwned>(&self, key: &str, value: T) -> Result<(), CacheError> where T: Serialize + Send {
        let mut conn = self.conn.clone();
        let value_vec = serde_json::to_vec(&value).map_err(CacheError::SerializationError)?;
        conn.set(key, value_vec).await.map_err(CacheError::RedisError)?;
        Ok(())
    }

    async fn del(&self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        conn.del(key).await.map_err(CacheError::RedisError)?;
        Ok(())
    }
}