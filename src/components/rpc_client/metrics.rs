use prometheus::Histogram;
use prometheus::HistogramOpts;
use prometheus::IntCounter;
use prometheus::Registry;

#[derive(Clone)]
pub struct RPCMetrics {
    pub call_duration: Histogram,
    pub total_request: IntCounter,
    pub hit_cache: IntCounter,
}

impl RPCMetrics {
    pub fn new(registry: &Registry, name: &str) -> Self {
        let opts = HistogramOpts::new(name, "duration of rpc call");
        let duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(duration.clone()))
            .unwrap_or_default();

        let counter = IntCounter::new(name, "count rpc call").unwrap();
        registry
            .register(Box::new(counter.clone()))
            .unwrap_or_default();

        let hit_cache = IntCounter::new(name, "count rpc call hit cache").unwrap();
        registry
            .register(Box::new(hit_cache.clone()))
            .unwrap_or_default();

        Self {
            call_duration: duration,
            total_request: counter,
            hit_cache,
        }
    }
}
