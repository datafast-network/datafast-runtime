use prometheus::IntCounter;
use prometheus::Registry;

#[derive(Clone)]
pub struct DatabaseMetrics {
    pub cache_hit: IntCounter,
    pub cache_miss: IntCounter,
    pub extern_db_write: IntCounter,
    pub extern_db_load: IntCounter,
}

impl DatabaseMetrics {
    pub fn new(registry: &Registry) -> Self {
        let cache_hit = IntCounter::new("cache_hit", "cache-hit count").unwrap();
        registry.register(Box::new(cache_hit.clone())).unwrap();

        let cache_miss = IntCounter::new("cache_miss", "cache-miss count").unwrap();
        registry.register(Box::new(cache_miss.clone())).unwrap();

        let extern_db_write = IntCounter::new("extern_db_write", "extern db write count").unwrap();
        registry
            .register(Box::new(extern_db_write.clone()))
            .unwrap();

        let extern_db_load = IntCounter::new("extern_db_load", "extern db load count").unwrap();
        registry.register(Box::new(extern_db_load.clone())).unwrap();

        Self {
            cache_hit,
            cache_miss,
            extern_db_write,
            extern_db_load,
        }
    }
}
