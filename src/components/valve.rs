use crate::config::ValveConfig;
use crate::info;
use std::sync::Arc;
use std::sync::RwLock;

pub struct InnerValve {
    finished: u64,
    downloaded: u64,
    cfg: ValveConfig,
}

#[derive(Clone)]
pub struct Valve(Arc<RwLock<InnerValve>>);

impl Valve {
    pub fn new(cfg: &ValveConfig) -> Self {
        let this = InnerValve {
            finished: 0,
            downloaded: 0,
            cfg: cfg.to_owned(),
        };
        Valve(Arc::new(RwLock::new(this)))
    }

    pub fn get_wait(&self) -> u64 {
        self.0.read().unwrap().cfg.wait_time
    }

    pub fn get_downloaded(&self) -> u64 {
        self.0.read().unwrap().downloaded
    }

    pub fn should_continue(&self) -> bool {
        let this = self.0.read().unwrap();

        if this.cfg.allowed_lag == 0 {
            return true;
        }

        let result = this.downloaded - this.finished < this.cfg.allowed_lag;

        info!(
            Valve,
            format!("Should continue? {result}");
            downloaded => this.downloaded,
            finished => this.finished,
            lag => this.downloaded - this.finished,
            allowed_lag => this.cfg.allowed_lag
        );

        result
    }

    pub fn set_finished(&self, finished_block: u64) {
        let mut this = self.0.write().unwrap();
        this.finished = finished_block;
    }

    pub fn set_downloaded(&self, downloaded_block: u64) {
        let mut this = self.0.write().unwrap();
        this.downloaded = downloaded_block;
    }
}
