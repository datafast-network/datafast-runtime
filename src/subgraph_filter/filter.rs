use super::event_filter::EventFilter;

use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::event::EthereumEventData;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::errors::FilterError;
use ethabi::Address;
use web3::types::Log;
use web3::types::H160;

pub type FilterResult<T> = Result<T, FilterError>;

pub enum FilterData {
    EthereumBlockData(EthereumBlockData),
    EthereumLogs(Vec<Log>),
    EthereumTransactions(Vec<EthereumTransactionData>),
    EthereumEventData(Vec<EthereumEventData>),
}
impl FilterData {
    pub fn get_logs(&self) -> Vec<Log> {
        match self {
            FilterData::EthereumLogs(logs) => logs.clone(),
            _ => panic!("Invalid FilterData for Logs"),
        }
    }
}

#[async_trait::async_trait]
pub trait SubgraphFilter {
    async fn filter_log(&self, filter_data: FilterData) -> FilterResult<FilterData>;

    fn get_contract(&self) -> ethabi::Contract;

    fn get_address(&self) -> &H160;
}

#[derive(Debug, Clone)]
pub enum FilterTypes {
    LogEvent(EventFilter),
}
#[async_trait::async_trait]
impl SubgraphFilter for FilterTypes {
    async fn filter_log(&self, filter_data: FilterData) -> FilterResult<FilterData> {
        match self {
            FilterTypes::LogEvent(filter) => filter.filter_log(filter_data).await,
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
