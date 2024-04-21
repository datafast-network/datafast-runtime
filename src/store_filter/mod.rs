mod cache_db;

pub use cache_db::{create_redis_cache, CacheDb, CacheTrait};
use crate::errors::{CacheError};

#[derive(Clone)]
pub struct StoreFilter {
    cache: CacheDb,
}

impl From<CacheDb> for StoreFilter {
    fn from(cache: CacheDb) -> Self {
        StoreFilter { cache }
    }
}

impl StoreFilter {
    pub fn get(&self, key: &str) -> Result<Vec<u8>, CacheError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let result = self.cache.get(key).await?;
                Ok(result)
            })
        })
    }

    pub fn set(&self, key: &str, value: Vec<u8>) -> Result<(), CacheError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                self.cache.set(key, value).await?;
                Ok(())
            })
        })
    }

    pub fn remove(&self, key: &str) -> Result<(), CacheError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                self.cache.del(key).await?;
                Ok(())
            })
        })
    }
}