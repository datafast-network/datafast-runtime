mod ethereum_filter;
mod utils;

use crate::common::ABIs;
use crate::common::BlockDataMessage;
use crate::common::Chain;
use crate::common::Datasource;
use crate::common::FilteredDataMessage;
use crate::errors::FilterError;
use ethereum_filter::EthereumFilter;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;

pub trait DataFilterTrait: Sized {
    fn handle_serialize_message(
        &self,
        data: BlockDataMessage,
    ) -> Result<FilteredDataMessage, FilterError>;
}

#[derive(Debug)]
pub enum DataFilter {
    Ethereum(EthereumFilter),
}

impl DataFilter {
    pub fn filter_multi(
        &self,
        messages: Vec<BlockDataMessage>,
    ) -> Result<Vec<FilteredDataMessage>, FilterError> {
        let result = messages
            .into_par_iter()
            .map(|m| self.handle_serialize_message(m).unwrap())
            .collect::<Vec<_>>();

        Ok(result)
    }

    pub fn new(
        chain: Chain,
        datasources: Vec<Datasource>,
        abis: ABIs,
    ) -> Result<Self, FilterError> {
        let filter = match chain {
            Chain::Ethereum => DataFilter::Ethereum(EthereumFilter::new(datasources, abis)),
        };
        Ok(filter)
    }
}

impl DataFilterTrait for DataFilter {
    fn handle_serialize_message(
        &self,
        data: BlockDataMessage,
    ) -> Result<FilteredDataMessage, FilterError> {
        match self {
            DataFilter::Ethereum(filter) => filter.handle_serialize_message(data),
        }
    }
}
