#[cfg(feature = "deltalake")]
mod delta;
mod metrics;
#[cfg(any(feature = "pubsub_compress", feature = "pubsub"))]
mod pubsub;

#[cfg(all(feature = "pubsub_compress", feature = "pubsub"))]
use pubsub::PubSubSource;

use super::Valve;
use crate::common::BlockDataMessage;
use crate::common::Chain;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
#[cfg(feature = "deltalake")]
use delta::DeltaClient;
#[cfg(feature = "deltalake")]
use delta::DeltaEthereumBlocks;
use kanal::AsyncSender;
use prometheus::Registry;

enum Source {
    #[cfg(feature = "deltalake")]
    Delta(DeltaClient),
    #[cfg(all(feature = "pubsub_compress", feature = "pubsub"))]
    PubSub(PubSubSource),
}

pub struct BlockSource {
    source: Source,
    chain: Chain,
}

impl BlockSource {
    pub async fn new(
        config: &Config,
        start_block: u64,
        registry: &Registry,
    ) -> Result<Self, SourceError> {
        let source = match &config.source {
            #[cfg(feature = "deltalake")]
            SourceTypes::Delta(delta_cfg) => {
                Source::Delta(DeltaClient::new(delta_cfg.to_owned(), start_block, registry).await?)
            }
            #[cfg(all(feature = "pubsub", feature = "pubsub_compress"))]
            SourceTypes::PubSub {
                topic,
                sub_id,
            } => Source::PubSub(
                PubSubSource::new(topic.clone(), sub_id.clone()).await?,
            ),
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
            #[cfg(all(feature = "pubsub", feature = "pubsub_compress"))]
            Source::PubSub(source) => match self.chain {
                Chain::Ethereum => source.subscribe(sender).await?,
            },
        };

        Ok(())
    }
}
