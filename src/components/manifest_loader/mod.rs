mod local;

use crate::common::Source;
use crate::common::{BlockPtr, Datasource};
use crate::errors::ManifestLoaderError;
use crate::info;
use crate::schema_lookup::SchemaLookup;
use async_trait::async_trait;
use local::LocalFileLoader;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait LoaderTrait: Sized {
    async fn load_schema(&mut self) -> Result<(), ManifestLoaderError>;

    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError>;

    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError>;

    // Load-Wasm is lazy, we only execute it when we need it
    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError>;

    fn get_abis(&self) -> HashMap<String, Value>;

    fn get_schema(&self) -> SchemaLookup;

    fn get_sources(&self) -> Vec<Source>;

    async fn create_datasource(
        &mut self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError>;
}

enum ManifestLoader {
    Local(LocalFileLoader),
}

impl ManifestLoader {
    async fn new(path: &str) -> Result<Self, ManifestLoaderError> {
        let parts = path
            .split("://")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        let protocol = parts[0].clone();

        match protocol.as_str() {
            "fs" => {
                let local_path = format!("/{}", parts[1]);
                info!(
                    ManifestLoader,
                    "Using LocalFile Loader, loading subgraph build bundle";
                    build_bundle_path => local_path
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

    pub async fn create_datasource(
        &mut self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError> {
        match self {
            Self::Local(loader) => loader.create_datasource(name, params, block_ptr).await,
        }
    }
}

#[derive(Clone)]
pub struct ManifestAgent {
    loader: Arc<Mutex<ManifestLoader>>,
}

impl ManifestAgent {
    pub async fn new(path: &str) -> Result<Self, ManifestLoaderError> {
        let loader = ManifestLoader::new(path).await?;
        Ok(ManifestAgent {
            loader: Arc::new(Mutex::new(loader)),
        })
    }

    pub async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        let loader = self.loader.lock().unwrap();
        loader.load_wasm(datasource_name).await
    }

    pub fn get_abis(&self) -> HashMap<String, Value> {
        let loader = self.loader.lock().unwrap();
        loader.get_abis()
    }

    pub fn get_schema(&self) -> SchemaLookup {
        let loader = self.loader.lock().unwrap();
        loader.get_schema()
    }

    pub fn get_sources(&self) -> Vec<Source> {
        let loader = self.loader.lock().unwrap();
        loader.get_sources()
    }

    pub fn datasources(&self) -> Vec<Datasource> {
        let loader = self.loader.lock().unwrap();
        loader.datasources()
    }

    pub fn create_datasource(
        &self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError> {
        use std::thread;
        let loader = self.loader.clone();
        let name = name.to_owned();
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();

            rt.block_on(async move {
                let mut loader = loader.lock().unwrap();
                loader.create_datasource(&name, params, block_ptr).await
            })
        })
        .join()
        .unwrap()
    }
}
