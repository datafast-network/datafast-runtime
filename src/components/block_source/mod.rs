mod delta;
mod from_to;
mod metrics;
pub mod proto;
mod redis_source;

use super::Valve;
use crate::common::BlockDataMessage;
use crate::common::Chain;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use delta::DeltaClient;
use delta::DeltaEthereumBlocks;
use kanal::AsyncSender;
use prometheus::Registry;
use redis_source::RedisSource;

enum Source {
    Delta(DeltaClient),
    Redis(RedisSource),
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
            SourceTypes::Delta(delta_cfg) => {
                Source::Delta(DeltaClient::new(delta_cfg.to_owned(), start_block, registry).await?)
            }
            SourceTypes::Redis { redis_uri, channel } => {
                Source::Redis(RedisSource::new(redis_uri, channel).await?)
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
            },
            Source::Redis(source) => {
                source.subscribe(sender).await?
            }
        };

        Ok(())
    }
}
