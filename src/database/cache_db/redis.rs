use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use redis::aio::MultiplexedConnection;
use serde::de::DeserializeOwned;
use serde::{Serialize};
use crate::database::cache::ICache;
use crate::errors::CacheError;

pub struct RedisCache {
    client: Client,
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
                client,
                conn,
            }
        )
    }
}


#[async_trait]
impl ICache for RedisCache {
    async fn get<T>(&self, key: &str) -> Result<T, CacheError> where T: DeserializeOwned + Send {
        let mut conn = self.conn.clone();
        let value_vec: Vec<u8> = conn.get(key).await.map_err(CacheError::RedisError)?;
        serde_json::from_slice(&value_vec).map_err(CacheError::SerializationError)
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