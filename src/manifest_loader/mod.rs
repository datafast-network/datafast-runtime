mod local;

use crate::common::Datasource;
use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use local::LocalFileLoader;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait LoaderTrait: Sized {
    async fn new(path: &str) -> Result<Self, ManifestLoaderError>;
    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError>;
    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError>;
    // Load-Wasm is lazy, we only execute it when we need it
    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError>;
    fn get_abis(&self) -> &HashMap<String, serde_json::Value>;

    fn load_ethereum_contract(
        &self,
        datasource_name: &str,
    ) -> Result<ethabi::Contract, ManifestLoaderError> {
        let abi =
            self.get_abis()
                .get(datasource_name)
                .ok_or(ManifestLoaderError::InvalidDataSource(
                    datasource_name.to_owned(),
                ))?;
        serde_json::from_value(abi.clone())
            .map_err(|_| ManifestLoaderError::InvalidABI(datasource_name.to_owned()))
    }

    fn load_ethereum_contracts(
        &self,
    ) -> Result<HashMap<String, ethabi::Contract>, ManifestLoaderError> {
        let contracts = self
            .get_abis()
            .iter()
            .filter_map(
                |(source_name, abi)| match self.load_ethereum_contract(source_name) {
                    Ok(contract) => Some((source_name.clone(), contract)),
                    Err(_) => None,
                },
            )
            .collect();

        Ok(contracts)
    }
}
pub enum ManifestLoader {
    Local(LocalFileLoader),
}

#[async_trait]
impl LoaderTrait for ManifestLoader {
    async fn new(path: &str) -> Result<Self, ManifestLoaderError> {
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

    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError> {
        match self {
            ManifestLoader::Local(loader) => loader.load_yaml().await,
        }
    }

    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError> {
        match self {
            ManifestLoader::Local(loader) => loader.load_abis().await,
        }
    }

    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        match self {
            ManifestLoader::Local(loader) => loader.load_wasm(datasource_name).await,
        }
    }
    fn get_abis(&self) -> &HashMap<String, Value> {
        match self {
            ManifestLoader::Local(loader) => loader.get_abis(),
        }
    }
}

impl ManifestLoader {
    pub fn datasources(&self) -> Vec<Datasource> {
        match self {
            Self::Local(loader) => loader.subgraph_yaml.dataSources.to_vec(),
        }
    }

    pub fn get_abi(&self, datasource_name: &str, abi_name: &str) -> Option<serde_json::Value> {
        match self {
            Self::Local(loader) => loader.abis.get(datasource_name)?.get(abi_name).cloned(),
        }
    }
}
