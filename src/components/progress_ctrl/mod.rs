use crate::{
    common::{BlockPtr, Source},
    errors::ProgressCtrlError,
};

use super::database::Agent;

pub struct ProgressCtrl {
    db: Agent,
    last_block_ptr: Option<BlockPtr>,
    sources: Vec<Source>,
}

impl ProgressCtrl {
    pub async fn new(db: Agent, sources: Vec<Source>) -> Result<Self, ProgressCtrlError> {
        let last_block_ptr = db
            .get_last_processed_block_ptr()
            .await
            .map_err(ProgressCtrlError::LoadLastBlockPtrFail)?;
        let this = Self {
            db,
            last_block_ptr,
            sources,
        };
        Ok(this)
    }

    fn get_min_start_block(&self) -> u64 {
        let min_start_block = self.sources.iter().filter_map(|s| s.startBlock).min();
        return min_start_block.unwrap_or(0);
    }

    pub fn progress_check(&mut self, new_block_ptr: BlockPtr) -> Result<(), ProgressCtrlError> {
        match self.last_block_ptr {
            None => {
                let min_start_block = self.get_min_start_block();

                if new_block_ptr.number == min_start_block {
                    return Ok(());
                }

                Err(ProgressCtrlError::InvalidStartBlock((
                    min_start_block,
                    new_block_ptr.number,
                )))
            }
            Some(BlockPtr { number, hash }) => {
                todo!()
            }
        }
    }
}
