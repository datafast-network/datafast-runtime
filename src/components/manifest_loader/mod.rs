mod local;

use crate::common::Datasource;
use crate::common::Source;
use crate::errors::ManifestLoaderError;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;
use local::LocalFileLoader;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait LoaderTrait: Sized {
    async fn load_schema(&mut self) -> Result<(), ManifestLoaderError>;

    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError>;

    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError>;

    // Load-Wasm is lazy, we only execute it when we need it
    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError>;

    fn get_abis(&self) -> HashMap<String, serde_json::Value>;

    fn get_schema(&self) -> SchemaLookup;

    fn get_sources(&self) -> Vec<Source>;
}

pub enum ManifestLoader {
    Local(LocalFileLoader),
}

impl ManifestLoader {
    pub async fn new(path: &str) -> Result<Self, ManifestLoaderError> {
        let parts = path
            .split("://")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        let protocol = parts[0].clone();

        match protocol.as_str() {
            "fs" => {
                let local_path = format!("/{}", parts[1]);
                log::info!(
                    "Using LocalFile Loader, loading subgraph build bundle at: {}",
                    local_path
                );
                let loader = LocalFileLoader::new(&local_path).await?;
                Ok(ManifestLoader::Local(loader))
            }
            _ => {
                unimplemented!()
            }
        }
    }

    pub async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        match self {
            ManifestLoader::Local(loader) => loader.load_wasm(datasource_name).await,
        }
    }

    pub fn get_abis(&self) -> HashMap<String, Value> {
        match self {
            ManifestLoader::Local(loader) => loader.get_abis(),
        }
    }

    pub fn get_schema(&self) -> SchemaLookup {
        match self {
            ManifestLoader::Local(loader) => loader.get_schema(),
        }
    }

    pub fn get_sources(&self) -> Vec<Source> {
        match self {
            ManifestLoader::Local(loader) => loader.get_sources(),
        }
    }

    pub fn datasources(&self) -> Vec<Datasource> {
        match self {
            Self::Local(loader) => loader.subgraph_yaml.dataSources.to_vec(),
        }
    }

    pub fn load_ethereum_contracts(
        &self,
    ) -> Result<HashMap<String, ethabi::Contract>, ManifestLoaderError> {
        let abis = self.get_abis();
        let contracts = abis
            .into_iter()
            .map(|(source_name, abi)| {
                let contract = serde_json::from_value(abi.clone()).unwrap();
                (source_name, contract)
            })
            .collect();

        Ok(contracts)
    }
}
