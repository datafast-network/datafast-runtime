mod ethereum_filter;
mod utils;

use super::manifest_loader::ManifestLoader;
use crate::common::Chain;
use crate::errors::FilterError;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use ethereum_filter::EthereumFilter;

#[derive(Debug)]
pub enum SubgraphFilter {
    Ethereum(EthereumFilter),
}

impl SubgraphFilter {
    pub async fn run_sync(
        &self,
        msg: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        let result = self.handle_serialize_message(msg)?;
        Ok(result)
    }

    pub fn new(chain: Chain, manifest: &ManifestLoader) -> Result<Self, FilterError> {
        let filter = match chain {
            Chain::Ethereum => SubgraphFilter::Ethereum(EthereumFilter::new(chain, manifest)?),
        };
        Ok(filter)
    }
}

pub trait SubgraphFilterTrait: Sized {
    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError>;
}

impl SubgraphFilterTrait for SubgraphFilter {
    fn handle_serialize_message(
        &self,
        data: SerializedDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match self {
            SubgraphFilter::Ethereum(filter) => filter.handle_serialize_message(data),
        }
    }
}
