use ethabi::Contract;
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
    pub templates: Option<Vec<Datasource>>,
}

impl SubgraphYaml {
    pub fn abis(&self) -> HashMap<String, String> {
        let mut abis = HashMap::new();
        for ds in self.dataSources {
            for mapping_abi in ds.mapping.abis {
                abis.insert(mapping_abi.name, mapping_abi.file);
            }
        }
        for ds in self.templates.unwrap_or(vec![]) {
            for mapping_abi in ds.mapping.abis {
                abis.insert(mapping_abi.name, mapping_abi.file);
            }
        }
        abis
    }

    pub fn wasms(&self) -> HashMap<String, String> {
        let mut wasms = HashMap::new();
        for ds in self.dataSources {
            wasms.insert(ds.name, ds.mapping.file);
        }
        for ds in self.templates.unwrap_or(vec![]) {
            wasms.insert(ds.name, ds.mapping.file);
        }
        wasms
    }

    pub fn min_start_block(&self) -> u64 {
        self.dataSources
            .iter()
            .map(|ds| ds.source.startBlock.unwrap_or(0))
            .min()
            .unwrap_or(0)
    }
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
pub struct ABIs(HashMap<String, serde_json::Value>);

impl FromIterator<(String, serde_json::Value)> for ABIs {
    fn from_iter<I: IntoIterator<Item = (String, serde_json::Value)>>(mut iter: I) -> Self {
        Self(iter.into_iter().collect::<HashMap<_, _>>())
    }
}

impl ABIs {
    pub fn get(&self, name: &str) -> Option<serde_json::Value> {
        self.0.get(name).cloned()
    }

    pub fn get_contract(&self, name: &str) -> Option<Contract> {
        self.0
            .get(name)
            .cloned()
            .map(|v| serde_json::from_value(v).ok())
            .flatten()
    }

    pub fn insert(&mut self, name: String, abi: serde_json::Value) {
        self.0.insert(name, abi);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone, Default)]
pub struct WASMs(HashMap<String, Vec<u8>>);

impl FromIterator<(String, Vec<u8>)> for WASMs {
    fn from_iter<I: IntoIterator<Item = (String, Vec<u8>)>>(mut iter: I) -> Self {
        Self(iter.into_iter().collect::<HashMap<_, _>>())
    }
}

impl WASMs {
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.0.get(name).cloned()
    }

    pub fn insert(&mut self, name: String, wasm: Vec<u8>) {
        self.0.insert(name, wasm);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub struct DatasourceBundle {
    pub ds: Datasource,
    pub abi: serde_json::Value,
    pub wasm: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DatasourceBundles(HashMap<(String, Option<String>), DatasourceBundle>);

impl DatasourceBundles {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, name: &str, address: Option<String>) -> Option<DatasourceBundle> {
        self.0.get(&(name.to_owned(), address)).cloned()
    }

    pub fn add(&self, ds: DatasourceBundle) -> Result<(), String> {
        if self.0.contains_key(&(ds.ds.name, ds.ds.source.address)) {
            return Err("Datasource already exist".to_owned());
        }

        self.0.insert((ds.ds.name, ds.ds.source.address), ds);
        Ok(())
    }

    pub fn extend(&mut self, ds: DatasourceBundles) {
        self.0.extend(ds.0)
    }
}

impl From<(&Vec<Datasource>, &ABIs, &WASMs)> for DatasourceBundles {
    fn from((sources, abis, wasms): (&Vec<Datasource>, &ABIs, &WASMs)) -> Self {
        let bundles = sources
            .iter()
            .map(|ds| {
                let bundle = DatasourceBundle {
                    ds: ds.clone(),
                    abi: abis.get(&ds.name).unwrap(),
                    wasm: wasms.get(&ds.name).unwrap(),
                };
                ((ds.name.clone(), ds.source.address.clone()), bundle)
            })
            .collect::<HashMap<_, _>>();
        Self(bundles)
    }
}

impl Into<Vec<Datasource>> for DatasourceBundles {
    fn into(self) -> Vec<Datasource> {
        self.0
            .values()
            .cloned()
            .into_iter()
            .map(|ds| ds.ds)
            .collect()
    }
}
