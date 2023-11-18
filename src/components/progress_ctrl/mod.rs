use super::database::Agent;
use crate::common::BlockPtr;
use crate::common::Source;
use crate::errors::ProgressCtrlError;

pub struct ProgressCtrl {
    db: Agent,
    recent_block_ptrs: Vec<BlockPtr>,
    sources: Vec<Source>,
}

impl ProgressCtrl {
    pub async fn new(
        db: Agent,
        sources: Vec<Source>,
        reorg_threshold: u16,
    ) -> Result<Self, ProgressCtrlError> {
        let recent_block_ptrs = db
            .get_recent_block_pointers(reorg_threshold)
            .await
            .map_err(ProgressCtrlError::LoadLastBlockPtrFail)?;
        let this = Self {
            db,
            recent_block_ptrs,
            sources,
        };
        Ok(this)
    }

    fn get_min_start_block(&self) -> u64 {
        let min_start_block = self.sources.iter().filter_map(|s| s.startBlock).min();
        return min_start_block.unwrap_or(0);
    }

    pub async fn progress_check(
        &mut self,
        new_block_ptr: BlockPtr,
    ) -> Result<(), ProgressCtrlError> {
        match &self.recent_block_ptrs.last() {
            None => {
                let min_start_block = self.get_min_start_block();

                if new_block_ptr.number == min_start_block {
                    return Ok(());
                }

                Err(ProgressCtrlError::InvalidStartBlock(
                    min_start_block,
                    new_block_ptr.number,
                ))
            }
            Some(recent_block_ptrs) => {
                if recent_block_ptrs.is_parent(new_block_ptr.clone()) {
                    return Ok(());
                }

                // reorg or not?
                // Block gap: 8 - 9 - (missing 10) - 11
                if recent_block_ptrs.number + 1 < new_block_ptr.number {
                    return Err(ProgressCtrlError::BlockGap);
                }

                // reorg happen some where...
                // First, check if the new block's parent-hash is hash of any block
                // in the current chain within threshold
                let maybe_parent_block = self
                    .recent_block_ptrs
                    .iter()
                    .find(|b| b.hash == new_block_ptr.parent_hash)
                    .cloned();

                match maybe_parent_block {
                    None => {
                        // Reorg happened somewhere before this new-block, we should be waiting
                        return Err(ProgressCtrlError::PossibleReorg);
                    }
                    Some(_) => {
                        // This new-block is the reorg block,
                        // We will process this block after having discarded all the obsolete blocks
                        self.db.revert_from_block(new_block_ptr.number).await?;
                        Ok(())
                    }
                }
            }
        }
    }
}
