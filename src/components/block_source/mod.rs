mod delta;

use super::Valve;
use crate::common::Chain;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use crate::messages::BlockDataMessage;
use delta::DeltaClient;
use delta::DeltaEthereumBlocks;
use kanal::AsyncSender;

enum Source {
    Delta(DeltaClient),
}

pub struct BlockSource {
    source: Source,
    chain: Chain,
}

impl BlockSource {
    pub async fn new(config: &Config, start_block: u64) -> Result<Self, SourceError> {
        let source = match &config.source {
            SourceTypes::Delta(delta_cfg) => {
                Source::Delta(DeltaClient::new(delta_cfg.to_owned(), start_block).await?)
            }
        };
        Ok(Self {
            source,
            chain: config.chain.clone(),
        })
    }

    pub async fn run(
        self,
        sender: AsyncSender<Vec<BlockDataMessage>>,
        valve: Valve,
    ) -> Result<(), SourceError> {
        match self.source {
            Source::Delta(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => {
                        source.get_block_stream::<DeltaEthereumBlocks>(sender, valve)
                    }
                };
                query_blocks.await?
            }
        };

        Ok(())
    }
}
