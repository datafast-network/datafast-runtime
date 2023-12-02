use crate::common::BlockPtr;
use crate::common::Source;
use crate::database::DatabaseAgent;
use crate::errors::ProgressCtrlError;

#[derive(Clone)]
pub struct ProgressCtrl {
    db: DatabaseAgent,
    recent_block_ptrs: Vec<BlockPtr>,
    sources: Vec<Source>,
}

impl ProgressCtrl {
    pub async fn new(
        db: DatabaseAgent,
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

    pub fn get_min_start_block(&self) -> u64 {
        let min_start_block = self.sources.iter().filter_map(|s| s.startBlock).min();
        min_start_block.unwrap_or(0).max(
            self.recent_block_ptrs
                .last()
                .cloned()
                .map(|b| b.number + 1)
                .unwrap_or_default(),
        )
    }

    pub async fn check_block(&mut self, new_block_ptr: BlockPtr) -> Result<(), ProgressCtrlError> {
        match &self.recent_block_ptrs.last() {
            None => {
                let min_start_block = self.get_min_start_block();

                if new_block_ptr.number == min_start_block {
                    self.recent_block_ptrs.push(new_block_ptr);
                    return Ok(());
                }

                Err(ProgressCtrlError::InvalidStartBlock(
                    min_start_block,
                    new_block_ptr.number,
                ))
            }
            Some(recent_block_ptrs) => {
                if recent_block_ptrs.is_parent(new_block_ptr.clone()) {
                    self.recent_block_ptrs.push(new_block_ptr);
                    self.recent_block_ptrs.remove(0);
                    return Ok(());
                }

                // reorg or not?
                // Block gap: 8 - 9 - (missing 10) - 11
                if recent_block_ptrs.number + 1 < new_block_ptr.number {
                    return Err(ProgressCtrlError::BlockGap(
                        recent_block_ptrs.number,
                        new_block_ptr.number,
                    ));
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
                        Err(ProgressCtrlError::PossibleReorg)
                    }
                    Some(parent_block) => {
                        // This new-block is the reorg block,
                        // We will process this block after having discarded all the obsolete blocks
                        self.db.revert_from_block(new_block_ptr.number).await?;
                        self.recent_block_ptrs
                            .retain(|b| b.number > parent_block.number);
                        self.recent_block_ptrs.push(new_block_ptr);
                        Ok(())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use prometheus::default_registry;
    use rstest::rstest;

    #[rstest]
    fn test_progress_ctrl(#[values(None, Some(0), Some(1), Some(2))] start_block: Option<u64>) {
        env_logger::try_init().unwrap_or_default();

        let registry = default_registry();
        let db = DatabaseAgent::mock(registry);
        let sources = vec![Source {
            address: None,
            abi: "".to_owned(),
            startBlock: start_block,
        }];
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async move {
            let pc = ProgressCtrl::new(db, sources, 100).await.unwrap();
            assert!(pc.recent_block_ptrs.is_empty());
            let actual_start_block = pc.get_min_start_block();

            match start_block {
                None => assert_eq!(actual_start_block, 0),
                Some(block_number) => assert_eq!(actual_start_block, block_number),
            }
        });
    }
}
