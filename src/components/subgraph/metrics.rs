use prometheus::Histogram;
use prometheus::HistogramOpts;
use prometheus::IntCounter;
use prometheus::IntGauge;
use prometheus::Registry;

#[derive(Clone)]
pub struct SubgraphMetrics {
    pub block_process_duration: Histogram,
    pub block_process_counter: IntCounter,
    pub current_block_number: IntGauge,
}

impl SubgraphMetrics {
    pub fn new(registry: &Registry) -> Self {
        let block_process_counter =
            IntCounter::new("block_process_counter", "count block process").unwrap();
        registry
            .register(Box::new(block_process_counter.clone()))
            .unwrap();

        let opts = HistogramOpts::new("block_process_duration", "duration of block processing");
        let block_process_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(block_process_duration.clone()))
            .unwrap();

        let current_block_number =
            IntGauge::new("current_block_number", "current block being processed").unwrap();
        registry
            .register(Box::new(current_block_number.clone()))
            .unwrap();

        Self {
            block_process_duration,
            block_process_counter,
            current_block_number,
        }
    }
}
