use crate::asc::errors::AscError;
use kanal::SendError;
use thiserror::Error;
use wasmer::CompileError;
use wasmer::InstantiationError;
use wasmer::RuntimeError;

#[derive(Error, Debug)]
pub enum HostExportErrors {
    #[error("Somethig wrong: {0}")]
    Plain(String),
}

#[derive(Error, Debug)]
pub enum WasmHostError {
    #[error("Compiling failed: {0}")]
    WasmCompileError(#[from] CompileError),
    #[error("Wasm Instantiation Error: {0}")]
    WasmInstanceError(#[from] InstantiationError),
}

#[derive(Debug, Error)]
pub enum ManifestLoaderError {
    #[error("No datasource with id={0} exists")]
    InvalidDataSource(String),
}

#[derive(Debug, Error)]
pub enum SubgraphError {
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    AscError(#[from] AscError),
    #[error("Invalid datasource_id: {0}")]
    InvalidSourceID(String),
    #[error("Invalid handler_name: {0}")]
    InvalidHandlerName(String),
    #[error("Something wrong: {0}")]
    Plain(String),
}

#[derive(Debug, Error)]
pub enum DatabaseWorkerError {
    #[error("Invalid operation")]
    Invalid,
    #[error("Something wrong: {0}")]
    Plain(String),
    #[error("Result-reply sending failed: {0}")]
    SendReplyFailed(#[from] SendError),
}

#[derive(Debug, Error)]
pub enum SwrError {
    #[error(transparent)]
    ManifestLoader(#[from] ManifestLoaderError),
    #[error("Config load failed!")]
    ConfigLoadFail,
    #[error(transparent)]
    WasmHostError(#[from] WasmHostError),
    #[error(transparent)]
    SubgraphError(#[from] SubgraphError),
    #[error(transparent)]
    DatabaseWorkerError(#[from] DatabaseWorkerError),
}
