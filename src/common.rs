use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;

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
#[allow(non_snake_case)]
pub struct Mapping {
    pub kind: String,
    pub apiVersion: Version,
    pub entities: Vec<String>,
    pub abis: Vec<MappingABI>,
    pub eventHandlers: Option<Vec<EventHandler>>,
    pub blockHandlers: Option<Vec<BlockHandler>>,
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
    pub templates: Vec<Datasource>,
}

#[derive(Debug)]
pub enum HandlerTypes {
    EthereumBlock,
    EthereumEvent,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Ethereum,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Default, Serialize, Hash)]
pub struct BlockPtr {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
}

impl BlockPtr {
    pub fn is_parent(&self, child_block_ptr: &BlockPtr) -> bool {
        self.number == child_block_ptr.number - 1 && self.hash == child_block_ptr.parent_hash
    }
}

impl Display for BlockPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BlockPtr({}, hash=`{}`, parent_hash=`{}`)",
            self.number, self.hash, self.parent_hash
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct ABIList(HashMap<String, serde_json::Value>);

impl ABIList {
    pub fn get(&self, name: &str) -> Option<serde_json::Value> {
        self.0.get(name).cloned()
    }

    pub fn insert(&mut self, name: String, abi: serde_json::Value) {
        self.0.insert(name, abi);
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, serde_json::Value> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
