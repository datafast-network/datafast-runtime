mod ethereum_filter;
mod utils;

use super::manifest_loader::ManifestLoader;
use crate::common::Chain;
use crate::errors::FilterError;
use crate::messages::FilteredDataMessage;
use crate::messages::SerializedDataMessage;
use ethereum_filter::EthereumFilter;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use rayon::slice::ParallelSliceMut;

#[derive(Debug)]
pub enum SubgraphFilter {
    Ethereum(EthereumFilter),
}

impl SubgraphFilter {
    pub fn filter_multi(
        &self,
        messages: Vec<SerializedDataMessage>,
    ) -> Result<Vec<FilteredDataMessage>, FilterError> {
        let mut result = messages
            .into_par_iter()
            .map(|m| self.handle_serialize_message(m).unwrap())
            .collect::<Vec<_>>();

        result.par_sort_by_key(|m| m.get_block_ptr().number);
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
