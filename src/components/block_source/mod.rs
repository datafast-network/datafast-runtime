mod metrics;

use super::Valve;
use crate::common::BlockDataMessage;
use crate::common::Chain;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use kanal::AsyncSender;
use prometheus::Registry;

#[cfg(feature = "deltalake")]
mod delta;
#[cfg(feature = "deltalake")]
use delta::DeltaClient;
#[cfg(feature = "deltalake")]
use delta::DeltaEthereumBlocks;

#[cfg(feature = "pubsub")]
mod pubsub;
use crate::components::manifest::StartBlock;
#[cfg(feature = "pubsub")]
use pubsub::PubSubSource;

enum Source {
    #[cfg(feature = "deltalake")]
    Delta(DeltaClient),
    #[cfg(feature = "pubsub")]
    PubSub(PubSubSource),
}

pub struct BlockSource {
    source: Source,
    chain: Chain,
}

impl BlockSource {
    pub async fn new(
        config: &Config,
        start_block: StartBlock,
        registry: &Registry,
    ) -> Result<Self, SourceError> {
        let source = match &config.source {
            #[cfg(feature = "deltalake")]
            SourceTypes::Delta(delta_cfg) => match start_block {
                StartBlock::Number(block) => {
                    Source::Delta(DeltaClient::new(delta_cfg.to_owned(), block, registry).await?)
                }
                _ => return Err(SourceError::DeltaInvalidStartBlock),
            },
            #[cfg(feature = "pubsub")]
            SourceTypes::PubSub {
                sub_id,
                compression,
            } => Source::PubSub(PubSubSource::new(sub_id.clone(), compression.clone()).await?),
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
            #[cfg(feature = "deltalake")]
            Source::Delta(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => {
                        source.get_block_stream::<DeltaEthereumBlocks>(sender, valve)
                    }
                };
                query_blocks.await?
            }
            #[cfg(feature = "pubsub")]
            Source::PubSub(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => source.get_block_stream::<DeltaEthereumBlocks>(sender),
                };
                query_blocks.await?
            }
        };

        Ok(())
    }
}
