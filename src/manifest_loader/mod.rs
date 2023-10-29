use crate::config::Config;
use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use semver::Version;
use std::collections::HashSet;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct DataSource {
    pub name: String,
    pub chain: String,
    pub block_handlers: Vec<String>,
    pub event_handlers: Vec<(String, String)>,
    pub tx_handlers: Vec<(String, String)>,
    pub wasm_path: String,
    pub abis: Vec<(String, serde_json::Value)>,
    pub version: Version,
    pub entities: HashSet<String>,
}

#[async_trait]
pub trait ManifestLoader: Sized + Send {
    async fn new(cfg: &Config) -> Result<Self, ManifestLoaderError>;

    fn datasources(&self) -> Vec<DataSource>;

    fn get_datasource_by_id(
        &self,
        source_id: impl ToString,
    ) -> Result<DataSource, ManifestLoaderError> {
        self.datasources()
            .into_iter()
            .find(|source| source.name == source_id.to_string())
            .ok_or_else(|| ManifestLoaderError::InvalidDataSource(source_id.to_string()))
            .map(|s| s.to_owned())
    }

    async fn load_wasm(&self) -> Result<Vec<u8>, ManifestLoaderError>;
}
