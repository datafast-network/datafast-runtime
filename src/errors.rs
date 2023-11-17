use kanal::SendError;
use scylla::transport::errors::NewSessionError;
use scylla::transport::errors::QueryError;
use std::io;
use thiserror::Error;
use wasmer::CompileError;
use wasmer::InstantiationError;
use wasmer::RuntimeError;

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

impl From<BigNumberErr> for RuntimeError {
    fn from(value: BigNumberErr) -> Self {
        match value {
            BigNumberErr::Parser => RuntimeError::new("Parser Error"),
            BigNumberErr::OutOfRange(_) => RuntimeError::new("Out of range"),
            BigNumberErr::NumberTooBig => RuntimeError::new("Number too big"),
            BigNumberErr::ParseError(_) => RuntimeError::new("Parse Error"),
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

impl From<AscError> for RuntimeError {
    fn from(err: AscError) -> Self {
        RuntimeError::new(err.to_string())
    }
}

#[derive(Error, Debug)]
pub enum WasmHostError {
    #[error("Wasm Compiling failed: {0}")]
    Compile(#[from] CompileError),
    #[error("Wasm Instantiation Failed: {0}")]
    Instantiation(#[from] InstantiationError),
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
    #[error("Invalid schema")]
    SchemaParsingError,
}

#[derive(Debug, Error)]
pub enum SubgraphError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Asc(#[from] AscError),
    #[error("Invalid datasource_id: {0}")]
    InvalidSourceID(String),
    #[error("Invalid handler_name: {0}")]
    InvalidHandlerName(String),
    #[error("Migrate memory to db error")]
    MigrateDbError,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Entity data missing `ID` field")]
    MissingID,
    #[error("Entity data missing field: {0}")]
    MissingField(String),
    #[error("Invalid operation")]
    Invalid,
    #[error("Invalid data value for field `{0}`")]
    InvalidValue(String),
    #[error("No such entity `{0}`")]
    EntityTypeNotExists(String),
    #[error("No such entity `{0}` with id=`{1}`")]
    EntityIDNotExists(String, String),
    #[error("Something wrong: {0}")]
    Plain(String),
    #[error("Result-reply sending failed: {0}")]
    SendReplyFailed(#[from] SendError),
    #[error("Database Mutex-lock failed")]
    MutexLockFailed,
    #[error("BlockPointer is missing")]
    MissingBlockPtr,
    #[error("Wasm-Host sent an invalid request")]
    WasmSendInvalidRequest,
    #[error("Failed to init new Scylla session")]
    ScyllaNewSession(#[from] NewSessionError),
    #[error("Scylla Query Error: `{0}`")]
    ScyllaQuery(#[from] QueryError),
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
    #[error("WasmHost error = {0}")]
    WasmHost(#[from] WasmHostError),
    #[error("Send result failed: {0}")]
    ChannelSendFail(#[from] SendError),
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Send data failed: {0}")]
    ChannelSendFail(#[from] SendError),
    #[error("Nats error: {0}")]
    Nats(#[from] io::Error),
    #[error("Nats parse message data failed: {0}")]
    ParseMessageFail(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ProgressCtrlError {
    #[error("Load block-ptr failed")]
    LoadLastBlockPtrFail(#[from] DatabaseError),
    #[error("Not a valid start-block (require `{0}`, actual = `{1}`)")]
    InvalidStartBlock(u64, u64),
    #[error("Unexpected block gap")]
    BlockGap,
    #[error("Possible reorg")]
    PossibleReorg,
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
