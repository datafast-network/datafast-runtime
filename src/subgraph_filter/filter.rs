use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use crate::subgraph_filter::chain::EthereumBlockFilter;
use crate::subgraph_filter::chain::EthereumLogFilter;
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

pub trait FilterTrait {
    fn filter_events(
        &self,
        filter_data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError>;
}

impl FilterTrait for Filter {
    fn filter_events(
        &self,
        filter_data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match self {
            Filter::Ethereum(filter) => filter.filter_events(filter_data),
        }
    }
}

pub struct SubgraphFilter {
    filter: Filter,
}

impl SubgraphFilter {
    pub fn new(manifest: &ManifestLoader) -> Result<Self, FilterError> {
        //TODO: Create filter based on chain from manifest or env
        let ethereum_filter = EthereumLogFilter::new(manifest)?;
        Ok(Self {
            filter: Filter::Ethereum(ethereum_filter),
        })
    }

    pub async fn run(
        &self,
        data_receiver: AsyncReceiver<SerializedDataMessage>,
        event_sender: AsyncSender<FilteredDataMessage>,
    ) -> Result<(), FilterError> {
        while let Ok(filter_data) = data_receiver.recv().await {
            let result = self.filter.filter_events(filter_data)?;
            event_sender.send(result).await?;
        }
        Ok(())
    }
}
