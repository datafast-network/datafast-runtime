mod local;

use crate::common::ABIList;
use crate::common::BlockPtr;
use crate::common::Datasource;
use crate::common::Source;
use crate::errors::ManifestLoaderError;
use crate::info;
use crate::schema_lookup::SchemaLookup;
use local::LocalFileLoader;
use std::sync::Arc;
use std::sync::Mutex;

pub trait LoaderTrait {
    fn get_abis(&self) -> ABIList;
    fn get_schema(&self) -> SchemaLookup;
    fn get_sources(&self) -> Vec<Source>;
    fn get_wasm(&self, source_name: &str) -> Vec<u8>;

    fn create_datasource(
        &mut self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError>;

    fn datasources_and_templates(&self) -> Vec<Datasource>;
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
                let loader = LocalFileLoader::new(&local_path)?;
                Ok(ManifestLoader::Local(loader))
            }
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn get_abis(&self) -> ABIList {
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

    pub fn get_wasm(&self, source_name: &str) -> Vec<u8> {
        match self {
            ManifestLoader::Local(loader) => loader.get_wasm(source_name),
        }
    }

    pub fn datasources(&self) -> Vec<Datasource> {
        match self {
            Self::Local(loader) => loader.subgraph_yaml.dataSources.to_vec(),
        }
    }

    pub fn count_datasources(&self) -> usize {
        match self {
            Self::Local(loader) => loader.subgraph_yaml.dataSources.len(),
        }
    }

    pub fn datasource_and_templates(&self) -> Vec<Datasource> {
        match self {
            Self::Local(loader) => loader.datasources_and_templates(),
        }
    }

    pub fn create_datasource(
        &mut self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError> {
        match self {
            Self::Local(loader) => loader.create_datasource(name, params, block_ptr),
        }
    }
}

#[derive(Clone)]
pub struct ManifestAgent {
    loader: Arc<Mutex<ManifestLoader>>,
}

impl ManifestAgent {
    pub fn mock() -> Self {
        let loader = ManifestLoader::Local(LocalFileLoader::default());
        ManifestAgent {
            loader: Arc::new(Mutex::new(loader)),
        }
    }
    pub async fn new(path: &str) -> Result<Self, ManifestLoaderError> {
        let loader = ManifestLoader::new(path).await?;
        Ok(ManifestAgent {
            loader: Arc::new(Mutex::new(loader)),
        })
    }

    pub fn get_abis(&self) -> ABIList {
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

    pub fn get_wasm(&self, source_name: &str) -> Vec<u8> {
        let loader = self.loader.lock().unwrap();
        loader.get_wasm(source_name)
    }

    pub fn datasources(&self) -> Vec<Datasource> {
        let loader = self.loader.lock().unwrap();
        loader.datasources()
    }

    pub fn count_datasources(&self) -> usize {
        let loader = self.loader.lock().unwrap();
        loader.count_datasources()
    }

    pub fn datasource_and_templates(&self) -> Vec<Datasource> {
        let loader = self.loader.lock().unwrap();
        loader.datasource_and_templates()
    }

    pub fn create_datasource(
        &self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError> {
        let mut loader = self.loader.lock().unwrap();
        loader.create_datasource(&name, params, block_ptr)
    }
}
