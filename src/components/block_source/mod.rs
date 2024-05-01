mod delta;
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
use delta::DeltaClient;
#[cfg(feature = "deltalake")]
use delta::DeltaEthereumBlocks;


enum Source {
    #[cfg(feature = "deltalake")]
    Delta(DeltaClient),
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
        };

        Ok(())
    }
}
