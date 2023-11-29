mod delta;

use crate::common::Chain;
use crate::components::ProgressCtrl;
use crate::config::Config;
use crate::config::SourceTypes;
use crate::error;
use crate::errors::SourceError;
use crate::messages::SerializedDataMessage;
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
    pub async fn new(config: &Config, pctrl: ProgressCtrl) -> Result<Self, SourceError> {
        let start_block = pctrl.get_min_start_block();
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

    pub async fn run_async(
        self,
        sender: AsyncSender<Vec<SerializedDataMessage>>,
    ) -> Result<(), SourceError> {
        let result = match self.source {
            Source::Delta(source) => {
                let query_blocks = match self.chain {
                    Chain::Ethereum => source.get_block_stream::<DeltaEthereumBlocks>(sender),
                };
                query_blocks.await
            }
        };
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                error!(
                    BlockSource,
                    "Source error"; error => format!("{:?}", e)
                );
                Err(e)
            }
        }
    }
}
