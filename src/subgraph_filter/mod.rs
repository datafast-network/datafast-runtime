pub mod errors;
mod event_filter;
mod filter;

pub use filter::SubgraphFilter;

pub type FilterResult<T> = Result<T, errors::FilterError>;
