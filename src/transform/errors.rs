use crate::asc::errors::AscError;
use kanal::SendError;
use log::error;
use thiserror::Error;
use wasmer::RuntimeError;

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("No transformer function with name={0}")]
    InvalidFunctionName(String),
    #[error("Failed to allocate memory for input data")]
    InputAllocationFail(#[from] AscError),
    #[error("Transfor failed: {0}")]
    TransformFail(#[from] RuntimeError),
    #[error("Forwarding data fail")]
    ForwardDataFail(#[from] SendError),
    #[error("Export error {0}")]
    ExportError(#[from] wasmer::ExportError),
}
