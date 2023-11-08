mod chain;
mod data_source_reader;

use crate::common::Chain;
use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use chain::EthereumLogFilter;
use kanal::AsyncReceiver;
use kanal::AsyncSender;

#[derive(Debug, Clone)]
enum Filter {
    Ethereum(EthereumLogFilter),
}

impl Filter {
    fn filter_events(
        &self,
        filter_data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match self {
            Filter::Ethereum(filter) => match filter_data {
                SerializedDataMessage::Ethereum { block, logs, .. } => {
                    filter.filter_events(block, logs)
                }
            },
        }
    }
}

pub struct SubgraphFilter {
    filter: Filter,
}

impl SubgraphFilter {
    pub fn new(chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let filter = match chain {
            Chain::Ethereum => Filter::Ethereum(EthereumLogFilter::new(manifest)?),
        };
        Ok(Self { filter })
    }

    pub async fn run_async(
        &self,
        data_receiver: AsyncReceiver<SerializedDataMessage>,
        result_sender: AsyncSender<FilteredDataMessage>,
    ) -> Result<(), FilterError> {
        while let Ok(filter_data) = data_receiver.recv().await {
            let result = self.filter.filter_events(filter_data)?;
            result_sender.send(result).await?;
        }
        Ok(())
    }
}
