use deltalake::datafusion::error::DataFusionError;
use deltalake::DeltaTableError;
use kanal::SendError;
use std::io;
use thiserror::Error;
use wasmer::CompileError;
use wasmer::MemoryAccessError;
use wasmer::RuntimeError;

#[cfg(feature = "scylla")]
use scylla::transport::errors as ScyllaError;

#[cfg(feature = "mongo")]
use mongodb::error as MongoError;

#[cfg(feature = "pubsub")]
use google_cloud_pubsub::client::Error as PubSubError;

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
    #[error(transparent)]
    WasmMemoryAccessError(#[from] MemoryAccessError),
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
    #[error("Create datasource failed")]
    CreateDatasourceFail,
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
    #[error("Create source failed: `{0}`")]
    CreateSourceFail(String),
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

    #[cfg(feature = "scylla")]
    #[error("Init failed")]
    ScyllaNewSession(#[from] ScyllaError::NewSessionError),

    #[cfg(feature = "scylla")]
    #[error("Query Error: `{0}`")]
    ScyllaQuery(#[from] ScyllaError::QueryError),

    #[cfg(feature = "mongo")]
    #[error("Init failed")]
    MongoDBInit(#[from] MongoError::Error),
}

#[derive(Debug, Error)]
pub enum FilterError {}

#[derive(Debug, Error)]
pub enum SerializerError {
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
    #[error("Failed to connect to Trino")]
    TrinoConnectionFail,
    #[error("Serialize from Trino row failed")]
    TrinoSerializeFail,
    #[error("Trino Query Failed")]
    TrinoQueryFail,
    #[error("DeltaTable Error")]
    DeltaTableError(#[from] DeltaTableError),
    #[error("DataFusion Error")]
    DataFusionError(#[from] DataFusionError),
    #[error("DeltaLake RecordBatch serialization error")]
    DeltaSerializationError,
    #[error("No blocks found from Delta")]
    DeltaEmptyData,
    #[cfg(feature = "pubsub")]
    #[error("PubSub error {0}")]
    PubSubError(#[from] PubSubError),
    #[cfg(feature = "pubsub")]
    #[error("PubSub decode message error {0}")]
    PubSubDecodeError(String)
}

#[derive(Debug, Error)]
pub enum RPCError {
    #[error("ABI is not valid")]
    BadABI,
    #[error("Contract call failed")]
    ContractCallFail,
    #[error("Function not found")]
    FunctionNotFound,
    #[error("Function Signature not found")]
    SignatureNotFound,
    #[error("Invalid Argument")]
    InvalidArguments,
    #[error("Data encoding failed")]
    DataEncodingFail,
    #[error("Data decoding failed")]
    DataDecodingFail,
    #[error("Chain not recognized")]
    InvalidChain,
    #[error("call reverted: {0}")]
    Revert(String),
    #[error("Get latest-block failed")]
    GetLatestBlockFail,
}

#[derive(Debug, Error)]
pub enum MainError {
    #[error("database error: `{0}`")]
    Database(#[from] DatabaseError),
    #[error("subgraph error: `{0}`")]
    Subgraph(#[from] SubgraphError),
    #[error("filter error: `{0}`")]
    Filter(#[from] FilterError),
}
