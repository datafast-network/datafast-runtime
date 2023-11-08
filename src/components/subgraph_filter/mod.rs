mod ethereum_filter;
mod utils;

use super::manifest_loader::ManifestLoader;
use crate::common::Chain;
use crate::errors::FilterError;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use ethereum_filter::EthereumFilter;
use kanal::AsyncReceiver;
use kanal::AsyncSender;

#[derive(Debug)]
pub enum SubgraphFilter {
    Ethereum(EthereumFilter),
}

impl SubgraphFilter {
    pub async fn run_async(
        &self,
        data_receiver: AsyncReceiver<SerializedDataMessage>,
        result_sender: AsyncSender<FilteredDataMessage>,
    ) -> Result<(), FilterError> {
        while let Ok(filter_data) = data_receiver.recv().await {
            let result = self.handle_serialize_message(filter_data)?;
            result_sender.send(result).await?;
        }
        Ok(())
    }
}

pub trait SubgraphFilterTrait: Sized {
    fn new(chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError>;
    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError>;
}

impl SubgraphFilterTrait for SubgraphFilter {
    fn new(chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let filter = match chain {
            Chain::Ethereum => SubgraphFilter::Ethereum(EthereumFilter::new(chain, manifest)?),
        };
        Ok(filter)
    }

    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match self {
            SubgraphFilter::Ethereum(filter) => filter.handle_serialize_message(data),
        }
    }
}
