use prometheus::Histogram;
use prometheus::HistogramOpts;
use prometheus::HistogramVec;
use prometheus::IntCounter;
use prometheus::IntGauge;
use prometheus::Registry;

#[derive(Clone)]
pub struct SubgraphMetrics {
    pub eth_event_process_duration: HistogramVec,
    pub eth_trigger_counter: IntCounter,
    pub block_process_duration: Histogram,
    pub current_block_number: IntGauge,
    pub datasource_creation_counter: IntCounter,
    pub datasource_creation_duration: Histogram,
}

impl SubgraphMetrics {
    pub fn new(registry: &Registry) -> Self {
        let opts = HistogramOpts::new("eth_event_process_duration", "duration of event processing");
        let eth_event_process_duration =
            HistogramVec::new(opts, &["datasource", "handler"]).unwrap();
        registry
            .register(Box::new(eth_event_process_duration.clone()))
            .unwrap();

        let eth_trigger_counter =
            IntCounter::new("eth_trigger_counter", "count eth block triggers").unwrap();
        registry
            .register(Box::new(eth_trigger_counter.clone()))
            .unwrap_or_default();

        let opts = HistogramOpts::new("block_process_duration", "duration of block processing");
        let block_process_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(block_process_duration.clone()))
            .unwrap_or_default();

        let current_block_number =
            IntGauge::new("current_block_number", "current block being processed").unwrap();
        registry
            .register(Box::new(current_block_number.clone()))
            .unwrap_or_default();

        let datasource_creation_counter = IntCounter::new(
            "datasource_creation_counter",
            "count number of datasource re-creations",
        )
        .unwrap();
        registry
            .register(Box::new(datasource_creation_counter.clone()))
            .unwrap_or_default();

        let opts = HistogramOpts::new(
            "datasource_creation_duration",
            "duration of creating datasources",
        );
        let datasource_creation_duration = Histogram::with_opts(opts).unwrap();
        registry
            .register(Box::new(datasource_creation_duration.clone()))
            .unwrap_or_default();

        Self {
            block_process_duration,
            eth_event_process_duration,
            eth_trigger_counter,
            current_block_number,
            datasource_creation_counter,
            datasource_creation_duration,
        }
    }
}
