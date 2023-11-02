use super::errors::FilterError;
use super::event_filter::EventFilter;
use super::event_filter::SubgraphLogData;
use crate::ingestor_data as pb;

use ethabi::Address;
use web3::types::H160;

pub type FilterResult<T> = Result<T, FilterError>;

#[async_trait::async_trait]
pub trait SubgraphFilter {
    async fn filter_log(
        &self,
        block_data: &pb::ethereum::Block,
    ) -> FilterResult<Vec<SubgraphLogData>>;

    fn get_contract(&self) -> ethabi::Contract;

    fn get_address(&self) -> &H160;
}

#[derive(Debug, Clone)]
pub enum FilterTypes {
    LogEvent(EventFilter),
}
#[async_trait::async_trait]
impl SubgraphFilter for FilterTypes {
    async fn filter_log(
        &self,
        block_data: &pb::ethereum::Block,
    ) -> FilterResult<Vec<SubgraphLogData>> {
        match self {
            FilterTypes::LogEvent(filter) => filter.filter_log(block_data).await,
        }
    }

    fn get_contract(&self) -> ethabi::Contract {
        match self {
            FilterTypes::LogEvent(filter) => filter.get_contract(),
        }
    }

    fn get_address(&self) -> &Address {
        match self {
            FilterTypes::LogEvent(filter) => filter.get_address(),
        }
    }
}
