use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MappingABI {
    pub name: String,
    pub file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Mapping {
    pub kind: String,
    pub apiVersion: Version,
    pub entities: Vec<String>,
    pub abis: Vec<MappingABI>,
    pub eventHandlers: Option<Vec<HashMap<String, String>>>,
    pub blockHandlers: Option<Vec<HashMap<String, String>>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Datasource {
    pub kind: String,
    pub name: String,
    pub network: String,
    pub source: HashMap<String, String>,
    pub mapping: Mapping,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct SubgraphYaml {
    pub dataSources: Vec<Datasource>,
}
