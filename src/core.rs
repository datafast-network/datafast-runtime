use crate::errors::ManifestLoaderError;
use crate::errors::SwrError;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DataSource {
    block_handlers: Vec<String>,
    event_handlers: Vec<(String, String)>,
    tx_handlers: Vec<(String, String)>,
    wasm_path: String,
}

#[derive(Clone, Debug)]
pub struct SubgraphManifest {
    datasources: HashMap<String, DataSource>,
}

#[async_trait]
pub trait ManifestLoader: Sized {
    fn datasources(&self) -> HashMap<String, DataSource>;

    async fn new(cfg: dyn Config) -> Result<Self, ManifestLoaderError>;

    async fn load(&self) -> Result<(), ManifestLoaderError>;

    fn get_source(&self, source_id: impl ToString) -> Result<DataSource, ManifestLoaderError> {
        let sources = self.datasources();
        sources
            .get(&source_id.to_string())
            .ok_or_else(|| ManifestLoaderError::InvalidDataSource(source_id.to_string()))
            .map(|s| s.to_owned())
    }

    async fn load_wasm(&self) -> Result<Vec<u8>, ManifestLoaderError>;
}

#[async_trait]
pub trait Source: Sized {
    async fn new(cfg: dyn Config) -> Result<Self, SwrError>;
}

pub trait Subgraph {}

pub trait Config {}

pub trait Runner {}
