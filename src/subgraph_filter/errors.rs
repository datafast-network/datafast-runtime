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
    EthAbiError(#[from] ethabi::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Web3 error: {0}")]
    Web3Error(#[from] web3::Error),
}
