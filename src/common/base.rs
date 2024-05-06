use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MappingABI {
    pub name: String,
    pub file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct EventHandler {
    pub event: String,
    pub handler: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BlockHandler {
    pub filter: Option<String>,
    pub handler: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TransactionHandler {
    pub filter: Option<String>,
    pub handler: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct Mapping {
    pub kind: String,
    pub apiVersion: Version,
    pub entities: Vec<String>,
    pub abis: Vec<MappingABI>,
    pub eventHandlers: Option<Vec<EventHandler>>,
    pub blockHandlers: Option<Vec<BlockHandler>>,
    pub transactionHandlers: Option<Vec<TransactionHandler>>,
    pub file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Datasource {
    pub kind: String,
    pub name: String,
    pub network: String,
    pub source: Source,
    pub mapping: Mapping,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct Source {
    pub address: Option<String>,
    pub abi: String,
    pub startBlock: Option<u64>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default, Clone)]
#[allow(non_snake_case)]
pub struct SubgraphYaml {
    pub dataSources: Vec<Datasource>,
    pub templates: Option<Vec<Datasource>>,
}

#[derive(Debug)]
pub enum HandlerTypes {
    EthereumBlock,
    EthereumTransaction,
    EthereumEvent,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Ethereum,
}

#[derive(Debug, Default, Clone)]
pub struct ABIs(pub HashMap<String, serde_json::Value>);

#[derive(Debug, Clone, Default)]
pub struct WASMs(pub HashMap<String, Vec<u8>>);

#[derive(Debug, Clone)]
pub struct DatasourceBundle {
    pub ds: Datasource,
    pub abi: serde_json::Value,
    pub wasm: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct DatasourceBundles {
    pub ds: Vec<DatasourceBundle>,
    pub keys: HashSet<(String, Option<String>)>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Default, Serialize, Hash)]
pub struct BlockPtr {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum StartBlock {
    Number(u64),
    Latest,
}

pub type EntityType = String;
pub type EntityID = String;
pub type FieldName = String;
pub type RawEntity = HashMap<FieldName, Value>;

#[derive(Clone, Debug, Default, PartialOrd, PartialEq)]
pub enum ModeSchema {
    ReadOnly,
    #[default]
    ReadWrite,
}

impl FromStr for ModeSchema {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let read_mods = vec!["readonly", "read", "r"];
        let mut mode = ModeSchema::default();
        if read_mods.contains(&s) {
            mode = ModeSchema::ReadOnly
        }
        Ok(mode)
    }
}

#[derive(Clone, Debug)]
pub struct SchemaConfig {
    pub mode: ModeSchema,
    pub namespace: Option<String>,
    pub interval: Option<u64>,
}

impl SchemaConfig {
    pub fn writeable(&self) -> bool {
        return self.mode == ModeSchema::ReadWrite;
    }
}

impl Default for SchemaConfig {
    fn default() -> Self {
        SchemaConfig {
            mode: ModeSchema::default(),
            namespace: None,
            interval: None,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct FieldKind {
    pub kind: StoreValueKind,
    pub relation: Option<(EntityType, FieldName)>,
    pub list_inner_kind: Option<StoreValueKind>,
}

pub type Schema = BTreeMap<FieldName, FieldKind>;

impl From<Option<u64>> for StartBlock {
    fn from(block: Option<u64>) -> Self {
        match block {
            Some(block) => StartBlock::Number(block),
            None => StartBlock::Latest,
        }
    }
}

impl Display for StartBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartBlock::Number(block) => write!(f, "StartBlock({})", block),
            StartBlock::Latest => write!(f, "StartBlock(Latest)"),
        }
    }
}
