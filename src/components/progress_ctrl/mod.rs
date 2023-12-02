use crate::common::BlockPtr;
use crate::common::Source;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
pub enum ProgressCheckResult {
    OkToProceed,
    BlockAlreadyProcessed,
    UnexpectedBlock { expected: u64, received: u64 },
    MaybeReorg,
    ForkBlock,
    UnrecognizedBlock(BlockPtr),
}

#[derive(Clone)]
pub struct ProgressCtrl {
    recent_block_ptrs: VecDeque<BlockPtr>,
    sources: Vec<Source>,
    reorg_threshold: u16,
}

impl ProgressCtrl {
    pub fn new(
        recent_block_ptrs: Vec<BlockPtr>,
        sources: Vec<Source>,
        reorg_threshold: u16,
    ) -> Self {
        Self {
            recent_block_ptrs: VecDeque::from(recent_block_ptrs),
            sources,
            reorg_threshold,
        }
    }

    pub fn get_min_start_block(&self) -> u64 {
        let min_start_block = self.sources.iter().filter_map(|s| s.startBlock).min();
        min_start_block.unwrap_or(0).max(
            self.recent_block_ptrs
                .front()
                .cloned()
                .map(|b| b.number + 1)
                .unwrap_or_default(),
        )
    }

    pub fn check_block(&mut self, new_block_ptr: BlockPtr) -> ProgressCheckResult {
        match self.recent_block_ptrs.front() {
            None => {
                let min_start_block = self.get_min_start_block();

                if new_block_ptr.number == min_start_block {
                    self.recent_block_ptrs.push_front(new_block_ptr);
                    return ProgressCheckResult::OkToProceed;
                }

                return ProgressCheckResult::UnexpectedBlock {
                    expected: min_start_block,
                    received: new_block_ptr.number,
                };
            }
            Some(last_processed) => {
                if last_processed.is_parent(&new_block_ptr) {
                    self.recent_block_ptrs.push_front(new_block_ptr);
                    if self.recent_block_ptrs.len() > self.reorg_threshold as usize {
                        self.recent_block_ptrs.pop_back();
                    }
                    return ProgressCheckResult::OkToProceed;
                }

                if new_block_ptr.number > last_processed.number + 1 {
                    return ProgressCheckResult::UnexpectedBlock {
                        expected: last_processed.number + 1,
                        received: new_block_ptr.number,
                    };
                }

                if new_block_ptr.number < self.recent_block_ptrs.back().unwrap().number {
                    return ProgressCheckResult::UnrecognizedBlock(new_block_ptr.clone());
                }

                for block in self.recent_block_ptrs.iter() {
                    if *block == new_block_ptr {
                        return ProgressCheckResult::BlockAlreadyProcessed;
                    }

                    if block.is_parent(&new_block_ptr) {
                        return ProgressCheckResult::ForkBlock;
                    }
                }

                return ProgressCheckResult::MaybeReorg;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_progress(#[values(None, Some(0), Some(1), Some(2))] start_block: Option<u64>) {
        env_logger::try_init().unwrap_or_default();
        let sources = vec![Source {
            address: None,
            abi: "".to_owned(),
            startBlock: start_block,
        }];
        let reorg_threshold = 10;
        let mut pc = ProgressCtrl::new(vec![], sources, reorg_threshold);
        assert!(pc.recent_block_ptrs.is_empty());

        let actual_start_block = pc.get_min_start_block();

        match start_block {
            None => {
                assert_eq!(actual_start_block, 0);
                for n in 0..20 {
                    let result = pc.check_block(BlockPtr {
                        number: n,
                        hash: format!("n={n}"),
                        parent_hash: if n > 0 {
                            format!("n={}", n - 1)
                        } else {
                            "".to_string()
                        },
                    });
                    assert_eq!(result, ProgressCheckResult::OkToProceed);
                }
                assert_eq!(pc.recent_block_ptrs.len(), reorg_threshold as usize);
                assert_eq!(pc.recent_block_ptrs.front().unwrap().number, 19);
                assert_eq!(pc.recent_block_ptrs.back().unwrap().number, 10);

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 22,
                        hash: "".to_string(),
                        parent_hash: "".to_string()
                    }),
                    ProgressCheckResult::UnexpectedBlock {
                        expected: 20,
                        received: 22
                    }
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 21,
                        hash: "".to_string(),
                        parent_hash: "".to_string()
                    }),
                    ProgressCheckResult::UnexpectedBlock {
                        expected: 20,
                        received: 21
                    }
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 20,
                        hash: "".to_string(),
                        parent_hash: "".to_string()
                    }),
                    ProgressCheckResult::MaybeReorg,
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 19,
                        hash: "".to_string(),
                        parent_hash: "".to_string()
                    }),
                    ProgressCheckResult::MaybeReorg,
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 15,
                        hash: "n=15".to_string(),
                        parent_hash: "n=some-fork-block".to_string()
                    }),
                    ProgressCheckResult::MaybeReorg,
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 9,
                        hash: "".to_string(),
                        parent_hash: "".to_string(),
                    }),
                    ProgressCheckResult::UnrecognizedBlock(BlockPtr {
                        number: 9,
                        hash: "".to_string(),
                        parent_hash: "".to_string(),
                    }),
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 10,
                        hash: "n=10".to_string(),
                        parent_hash: "n=9".to_string(),
                    }),
                    ProgressCheckResult::BlockAlreadyProcessed
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 19,
                        hash: "n=19".to_string(),
                        parent_hash: "n=18".to_string(),
                    }),
                    ProgressCheckResult::BlockAlreadyProcessed
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 15,
                        hash: "n=15".to_string(),
                        parent_hash: "n=14".to_string(),
                    }),
                    ProgressCheckResult::BlockAlreadyProcessed
                );

                assert_eq!(
                    pc.check_block(BlockPtr {
                        number: 20,
                        hash: "n=20".to_string(),
                        parent_hash: "n=19".to_string(),
                    }),
                    ProgressCheckResult::OkToProceed
                );

                assert_eq!(
                    pc.recent_block_ptrs.front().cloned().unwrap(),
                    BlockPtr {
                        number: 20,
                        hash: "n=20".to_string(),
                        parent_hash: "n=19".to_string(),
                    }
                );

                assert_eq!(
                    pc.recent_block_ptrs.back().cloned().unwrap(),
                    BlockPtr {
                        number: 11,
                        hash: "n=11".to_string(),
                        parent_hash: "n=10".to_string(),
                    }
                );
            }
            Some(block_number) => assert_eq!(actual_start_block, block_number),
        }
    }
}
