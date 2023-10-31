use thiserror::Error;

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("Filter not found")]
    NotFound,
    #[error("Filter invalid address: {0}")]
    InvalidAddress(String),
    #[error("Init filter error: {0}")]
    InitializationError(String),
    #[error("Ethereum error: {0}")]
    EthereumError(#[from] ethabi::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
}
