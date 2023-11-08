use kanal::SendError;
use std::io;
use thiserror::Error;
use wasmer::CompileError;
use wasmer::InstantiationError;

#[derive(Error, Debug)]
pub enum BigIntOutOfRangeError {
    #[error("Cannot convert negative BigInt into type")]
    Negative,
    #[error("BigInt value is too large for type")]
    Overflow,
}

#[derive(Error, Debug)]
pub enum BigNumberErr {
    #[error("Parser Error")]
    Parser,
    #[error(transparent)]
    OutOfRange(#[from] BigIntOutOfRangeError),
    #[error("Number too big")]
    NumberTooBig,
    #[error(transparent)]
    ParseError(#[from] num_bigint::ParseBigIntError),
}

impl From<BigNumberErr> for wasmer::RuntimeError {
    fn from(value: BigNumberErr) -> Self {
        match value {
            BigNumberErr::Parser => wasmer::RuntimeError::new("Parser Error"),
            BigNumberErr::OutOfRange(_) => wasmer::RuntimeError::new("Out of range"),
            BigNumberErr::NumberTooBig => wasmer::RuntimeError::new("Number too big"),
            BigNumberErr::ParseError(_) => wasmer::RuntimeError::new("Parse Error"),
        }
    }
}

#[derive(Debug, Error)]
pub enum AscError {
    #[error("Size not fit")]
    SizeNotFit,
    #[error("Value overflow: {0}")]
    Overflow(u32),
    #[error("Error: {0}")]
    Plain(String),
    #[error("Bad boolean value: {0}")]
    IncorrectBool(usize),
    #[error("Size does not match")]
    SizeNotMatch,
    #[error("Maximum Recursion Depth reached!")]
    MaxRecursion,
    #[error(transparent)]
    BigNumberOutOfRange(#[from] BigNumberErr),
}

impl From<AscError> for wasmer::RuntimeError {
    fn from(err: AscError) -> Self {
        wasmer::RuntimeError::new(err.to_string())
    }
}

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
    #[error("Invalid subgraph.yaml: {0}")]
    InvalidSubgraphYAML(String),
    #[error("Invalid abi: {0}")]
    InvalidABI(String),
    #[error("Invalid WASM: {0}")]
    InvalidWASM(String),
    #[error("Invalid subgraph dir: {0}")]
    InvalidSubgraphDir(String),
}

#[derive(Debug, Error)]
pub enum SubgraphError {
    #[error(transparent)]
    RuntimeError(#[from] wasmer::RuntimeError),
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
pub enum FilterError {
    #[error(transparent)]
    EthAbiError(#[from] ethabi::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error(transparent)]
    Web3Error(#[from] web3::Error),
    #[error(transparent)]
    SendReplyFailed(#[from] SendError),
    #[error(transparent)]
    JsonParseError(#[from] serde_json::Error),
    #[error(transparent)]
    ManifestLoaderError(#[from] ManifestLoaderError),
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("No transformer function with name={0}")]
    InvalidFunctionName(String),
    #[error("Transform function returns no value")]
    TransformReturnNoValue,
    #[error("Transform RuntimeError: {0}")]
    RuntimeError(#[from] wasmer::RuntimeError),
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
pub enum SourceError {
    #[error("Send data failed: {0}")]
    ChannelSendError(#[from] SendError),
    #[error("Nats error: {0}")]
    NatsError(#[from] io::Error),
    #[error("Nats parse message data failed: {0}")]
    ParseMessageError(#[from] serde_json::Error),
}

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
    FilterError(#[from] FilterError),
    #[error(transparent)]
    SerializerError(#[from] SerializerError),
    #[error(transparent)]
    SourceErr(#[from] SourceError),
}
