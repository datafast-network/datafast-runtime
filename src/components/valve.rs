use crate::config::ValveConfig;
use crate::info;
use crate::messages::SerializedDataMessage;
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

        info!(
            Valve,
            format!("should continue download? {result}");
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

    pub fn set_downloaded(&self, blocks: &[SerializedDataMessage]) {
        let mut this = self.0.write().unwrap();
        if let Some(last_block) = blocks.last() {
            let last_block_number = last_block.get_block_ptr().number;
            if last_block_number > this.downloaded {
                this.downloaded = last_block_number;
            }
        }
    }
}
