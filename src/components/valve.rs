use crate::config::ValveConfig;
use crate::info;
use prometheus::IntGauge;
use prometheus::Registry;
use std::sync::Arc;
use std::sync::RwLock;

struct ValveMetrics {
    block_downloaded_counter: IntGauge,
    block_finished_counter: IntGauge,
}

impl ValveMetrics {
    fn new(registry: &Registry) -> Self {
        let block_downloaded_counter =
            IntGauge::new("block_downloaded_counter", "count block downloaded").unwrap();
        registry
            .register(Box::new(block_downloaded_counter.clone()))
            .unwrap_or_default();

        let block_finished_counter =
            IntGauge::new("block_finished_counter", "count block finished").unwrap();
        registry
            .register(Box::new(block_finished_counter.clone()))
            .unwrap_or_default();

        Self {
            block_finished_counter,
            block_downloaded_counter,
        }
    }
}

pub struct InnerValve {
    finished: u64,
    downloaded: u64,
    cfg: ValveConfig,
    metrics: ValveMetrics,
}

#[derive(Clone)]
pub struct Valve(Arc<RwLock<InnerValve>>);

impl Valve {
    pub fn new(cfg: &ValveConfig, registry: &Registry) -> Self {
        let this = InnerValve {
            finished: 0,
            downloaded: 0,
            cfg: cfg.to_owned(),
            metrics: ValveMetrics::new(registry),
        };
        Valve(Arc::new(RwLock::new(this)))
    }

    pub fn get_wait(&self) -> u64 {
        self.0.read().unwrap().cfg.wait_time
    }

    pub fn should_continue(&self) -> bool {
        let this = self.0.read().unwrap();

        if this.cfg.allowed_lag == 0 {
            return true;
        }

        if this.downloaded < this.finished {
            // WARN: it is complicated!
            return true;
        }

        let result = this.downloaded - this.finished <= this.cfg.allowed_lag;

        if this.cfg.allowed_lag > 0 {
            info!(
                Valve,
                format!("processing status");
                downloaded => this.downloaded,
                finished => this.finished,
                lag => this.downloaded - this.finished,
                allowed_lag => this.cfg.allowed_lag,
                continue_download => result
            );
        }

        result
    }

    pub fn set_finished(&self, finished_block: u64) {
        let mut this = self.0.write().unwrap();
        this.finished = finished_block;
        this.metrics
            .block_finished_counter
            .set(finished_block as i64);
    }

    pub fn set_downloaded(&self, block_number: u64) {
        let mut this = self.0.write().unwrap();
        this.downloaded = block_number;
        this.metrics
            .block_downloaded_counter
            .set(block_number as i64);
    }
}
