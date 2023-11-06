use crate::errors::FilterError;
use crate::manifest_loader::ManifestLoader;
use crate::messages::SubgraphOperationMessage;
use crate::subgraph_filter::chain::EthereumBlockFilter;
use crate::subgraph_filter::chain::EthereumFilter;
use kanal::AsyncSender;

#[derive(Debug, Clone)]
pub enum FilterData {
    Block(EthereumBlockFilter),
}
pub type FilterResult<T> = Result<T, FilterError>;

#[derive(Debug, Clone)]
pub enum FilterChain {
    Ethereum(EthereumFilter),
}

pub trait SubgraphFilter {
    fn filter_events(
        &self,
        filter_data: FilterData,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError>;
}

impl SubgraphFilter for FilterChain {
    fn filter_events(
        &self,
        filter_data: FilterData,
    ) -> Result<Vec<SubgraphOperationMessage>, FilterError> {
        match self {
            FilterChain::Ethereum(filter) => filter.filter_events(filter_data),
        }
    }
}

pub struct SubgraphFilterInstance {
    filter: FilterChain,
    event_sender: AsyncSender<SubgraphOperationMessage>,
}

impl SubgraphFilterInstance {
    pub fn new(
        manifest: &ManifestLoader,
        sender: AsyncSender<SubgraphOperationMessage>,
    ) -> Result<Self, FilterError> {
        let ethereum_filter = EthereumFilter::new(manifest);
        Ok(Self {
            filter: FilterChain::Ethereum(ethereum_filter),
            event_sender: sender,
        })
    }

    pub async fn filter_events(&self, filter_data: FilterData) -> Result<(), FilterError> {
        let events = self.filter.filter_events(filter_data)?;
        for event in events {
            self.event_sender.send(event).await?;
        }
        Ok(())
    }
}
