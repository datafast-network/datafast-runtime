use crate::subgraph_filter::filter::SubgraphFilter;

mod errors;
mod event_filter;
mod filter;

pub type FilterResult<T> = Result<T, errors::FilterError>;

#[derive(Debug, Clone)]
pub enum FilterTypes {
    LogEvent(event_filter::EventFilter),
}

impl SubgraphFilter for FilterTypes {
    fn get_contract(&self) -> ethabi::Contract {
        match self {
            FilterTypes::LogEvent(filter) => filter.get_contract(),
        }
    }

    fn get_address(&self) -> &ethabi::Address {
        match self {
            FilterTypes::LogEvent(filter) => filter.get_address(),
        }
    }
}
