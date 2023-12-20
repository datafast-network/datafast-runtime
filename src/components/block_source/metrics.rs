use prometheus::{Histogram, IntCounter};

#[derive(Clone)]
pub struct BlockSourceMetrics {
    pub block_source_query_duration: Histogram,
    pub block_source_query_count: IntCounter,
    pub block_source_serialized_duration: Histogram,
    pub block_source_total_blocks: IntCounter,
}

impl BlockSourceMetrics {
    pub fn new(registry: &prometheus::Registry) -> Self {
        let opts = prometheus::HistogramOpts::new(
            "block_source_query_duration",
            "duration of block source request",
        );
        let block_source_query_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(block_source_query_duration.clone()))
            .unwrap_or_default();

        let block_source_query_count =
            IntCounter::new("block_source_query_count", "block source request count").unwrap();
        registry
            .register(Box::new(block_source_query_count.clone()))
            .unwrap_or_default();

        let opts = prometheus::HistogramOpts::new(
            "block_source_serialized_duration",
            "duration of block source serialized data",
        );
        let block_source_serialized_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(block_source_serialized_duration.clone()))
            .unwrap_or_default();

        let block_source_total_blocks = IntCounter::new(
            "block_source_total_blocks_downloaded",
            "block source total blocks downloaded",
        )
        .unwrap();
        registry
            .register(Box::new(block_source_total_blocks.clone()))
            .unwrap_or_default();

        Self {
            block_source_query_duration,
            block_source_query_count,
            block_source_serialized_duration,
            block_source_total_blocks,
        }
    }
}
