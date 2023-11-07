mod chain;
mod data_source_reader;

use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use chain::EthereumBlockFilter;
use chain::EthereumLogFilter;
use kanal::AsyncReceiver;
use kanal::AsyncSender;

#[derive(Debug, Clone)]
pub enum FilterData {
    Events(EthereumBlockFilter),
}

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
            Filter::Ethereum(filter) => filter.filter_events(filter_data.into()),
        }
    }
}

pub struct SubgraphFilter {
    filter: Filter,
}

impl SubgraphFilter {
    pub fn new(manifest: ManifestLoader) -> Result<Self, FilterError> {
        //TODO: Create filter based on chain from manifest or env
        let ethereum_filter = EthereumLogFilter::new(manifest.clone())?;
        Ok(Self {
            filter: Filter::Ethereum(ethereum_filter),
        })
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
