mod ethereum_filter;
mod utils;

use crate::common::ABIs;
use crate::common::Chain;
use crate::common::Datasource;
use crate::errors::FilterError;
use crate::messages::BlockDataMessage;
use crate::messages::FilteredDataMessage;
use ethereum_filter::EthereumFilter;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use rayon::slice::ParallelSliceMut;

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
        let mut result = messages
            .into_par_iter()
            .map(|m| self.handle_serialize_message(m).unwrap())
            .collect::<Vec<_>>();

        result.par_sort_unstable_by_key(|m| m.get_block_ptr().number);
        Ok(result)
    }

    pub fn new(
        chain: Chain,
        datasources: Vec<Datasource>,
        abi_list: ABIs,
    ) -> Result<Self, FilterError> {
        let filter = match chain {
            Chain::Ethereum => DataFilter::Ethereum(EthereumFilter::new(datasources, abi_list)),
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
