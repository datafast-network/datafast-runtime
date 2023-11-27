mod delta;
mod trino;

use crate::common::Chain;
use crate::components::ProgressCtrl;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
use delta::DeltaClient;
use delta::DeltaEthereumBlocks;
use kanal::bounded_async;
use kanal::AsyncSender;
use trino::TrinoClient;
use trino::TrinoEthereumBlock;

enum Source {
    Trino(TrinoClient),
    Delta(DeltaClient),
}

pub struct BlockSource {
    source: Source,
    chain: Chain,
}

impl BlockSource {
    pub async fn new(config: &Config, pctrl: ProgressCtrl) -> Result<Self, SourceError> {
        let start_block = pctrl.get_min_start_block();
        let source = match &config.source {
            SourceTypes::Trino(trino_cfg) => {
                Source::Trino(TrinoClient::new(trino_cfg.to_owned(), start_block)?)
            }
            SourceTypes::Delta(delta_cfg) => {
                Source::Delta(DeltaClient::new(delta_cfg.to_owned(), start_block).await?)
            }
        };
        Ok(Self {
            source,
            chain: config.chain.clone(),
        })
    }

    pub async fn run_async(
        self,
        sender: AsyncSender<Vec<SerializedDataMessage>>,
    ) -> Result<(), SourceError> {
        match self.source {
            Source::Trino(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => source.get_block_stream::<TrinoEthereumBlock>(sender),
                };
                query_blocks.await?
            }
            Source::Delta(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => source.get_block_stream::<DeltaEthereumBlocks>(sender),
                };
                query_blocks.await?
            }
        };

        Ok(())
    }
}
