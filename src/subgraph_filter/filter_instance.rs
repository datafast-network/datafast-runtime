use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::SubgraphOperationMessage;
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

pub trait SubgraphFilter {
    fn filter_events(
        &self,
        filter_data: FilterData,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError>;
}

impl SubgraphFilter for Filter {
    fn filter_events(
        &self,
        filter_data: FilterData,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError> {
        match self {
            Filter::Ethereum(filter) => filter.filter_events(filter_data),
        }
    }
}

pub struct SubgraphFilterInstance {
    filter: Filter,
    data_receiver: AsyncReceiver<FilterData>,
    event_sender: AsyncSender<SubgraphOperationMessage>,
}

impl SubgraphFilterInstance {
    pub fn new(
        manifest: &ManifestLoader,
        event_sender: AsyncSender<SubgraphOperationMessage>,
        data_receiver: AsyncReceiver<FilterData>,
    ) -> Result<Self, FilterError> {
        //TODO: Create filter based on chain from manifest or env
        let ethereum_filter = EthereumLogFilter::new(manifest);
        Ok(Self {
            filter: Filter::Ethereum(ethereum_filter),
            event_sender,
            data_receiver,
        })
    }

    pub async fn run(&self) -> Result<(), FilterError> {
        while let Ok(filter_data) = self.data_receiver.recv().await {
            let events = self.filter.filter_events(filter_data)?;
            for event in events {
                self.event_sender.send(event).await?;
            }
        }
        Ok(())
    }
}
