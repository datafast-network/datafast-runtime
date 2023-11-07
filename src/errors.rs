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
    #[error("Invalid `build` dir: {0}")]
    InvalidBuildDir(String),
    #[error("Invalid build path: {0}")]
    InvalidBuildPath(String),
    #[error("Invalid subgraph.yaml: {0}")]
    InvalidSubgraphYAML(String),
    #[error("Invalid abi: {0}")]
    InvalidABI(String),
    #[error("Invalid WASM: {0}")]
    InvalidWASM(String),
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
pub enum DatabaseError {
    #[error("Entity data missing `ID` field")]
    MissingID,
    #[error("Invalid operation")]
    Invalid,
    #[error("Something wrong: {0}")]
    Plain(String),
    #[error("Result-reply sending failed: {0}")]
    SendReplyFailed(#[from] SendError),
    #[error("Database Mutex-lock failed")]
    MutexLockFailed,
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("No transformer function with name={0}")]
    InvalidFunctionName(String),
    #[error("Transform function returns no value")]
    TransformReturnNoValue,
    #[error("Transform RuntimeError: {0}")]
    RuntimeError(#[from] RuntimeError),
    #[error("Transform AscError: {0}")]
    AscError(#[from] AscError),
    #[error("Chain mismatched")]
    ChainMismatched,
    #[error("Missing Transform Wasm module")]
    MissingTransformWASM,
    #[error("Bad Transform Wasm module: {0}")]
    BadTransformWasm(String),
}

#[derive(Debug, Error)]
pub enum SerializerError {
    #[error(transparent)]
    TransformError(#[from] TransformError),
    #[error(transparent)]
    WasmHost(#[from] WasmHostError),
    #[error("Send result failed: {0}")]
    ChannelSendError(#[from] SendError),
}

#[derive(Debug, Error)]
pub enum SourceErr {}

#[derive(Debug, Error)]
pub enum SwrError {
    #[error(transparent)]
    ManifestLoader(#[from] ManifestLoaderError),
    #[error("Config load failed: {0}")]
    ConfigLoadFail(String),
    #[error(transparent)]
    WasmHostError(#[from] WasmHostError),
    #[error(transparent)]
    SubgraphError(#[from] SubgraphError),
    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),
    #[error(transparent)]
    SerializerError(#[from] SerializerError),
}
