use prometheus::Histogram;
use prometheus::HistogramOpts;
use prometheus::IntCounter;
use prometheus::Registry;

#[derive(Clone)]
pub struct RpcMetrics {
    pub block_level_cache_hit: IntCounter,
    pub block_level_cache_miss: IntCounter,
    pub chain_level_cache_hit: IntCounter,
    pub chain_level_cache_miss: IntCounter,
    pub rpc_request_duration: Histogram,
}

impl RpcMetrics {
    pub fn new(registry: &Registry) -> Self {
        let block_level_cache_hit =
            IntCounter::new("rpc_block_level_cache_hit", "rpc cache-hit count").unwrap();
        registry
            .register(Box::new(block_level_cache_hit.clone()))
            .unwrap_or_default();

        let block_level_cache_miss =
            IntCounter::new("rpc_block_level_cache_miss", "rpc cache-miss count").unwrap();
        registry
            .register(Box::new(block_level_cache_miss.clone()))
            .unwrap_or_default();

        let chain_level_cache_hit =
            IntCounter::new("rpc_block_level_cache_hit", "rpc cache-hit count").unwrap();
        registry
            .register(Box::new(chain_level_cache_hit.clone()))
            .unwrap_or_default();

        let chain_level_cache_miss =
            IntCounter::new("rpc_block_level_cache_miss", "rpc cache-miss count").unwrap();
        registry
            .register(Box::new(chain_level_cache_miss.clone()))
            .unwrap_or_default();

        let opts = HistogramOpts::new("rpc_request_duration", "duration of rpc request");
        let rpc_request_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(rpc_request_duration.clone()))
            .unwrap_or_default();

        Self {
            block_level_cache_hit,
            block_level_cache_miss,
            chain_level_cache_hit,
            chain_level_cache_miss,
            rpc_request_duration,
        }
    }
}
