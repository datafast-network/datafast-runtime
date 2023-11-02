mod local;

use crate::common::Datasource;
use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use local::LocalFileLoader;
use log;

#[async_trait]
pub trait LoaderTrait: Sized {
    async fn new(path: &str) -> Result<Self, ManifestLoaderError>;
    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError>;
    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError>;
    // Load-Wasm is lazy, we only execute it when we need it
    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError>;
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
}

impl ManifestLoader {
    pub fn datasources(&self) -> Vec<Datasource> {
        match self {
            Self::Local(loader) => loader.subgraph_yaml.dataSources.to_vec(),
        }
    }
}
