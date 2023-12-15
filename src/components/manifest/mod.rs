mod local;

use crate::common::*;
use crate::error;
use crate::errors::ManifestLoaderError;
use crate::schema_lookup::SchemaLookup;
use local::LocalFileLoader;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct ManifestBundle {
    subgraph_yaml: SubgraphYaml,
    templates: DatasourceBundles,
    abis: ABIs,
    wasms: WASMs,
    schema: SchemaLookup,
    datasources: DatasourceBundles,
}

#[derive(Clone, Default)]
pub struct ManifestAgent(Arc<RwLock<ManifestBundle>>);

impl ManifestAgent {
    pub async fn new(subgraph_path: &str) -> Result<Self, ManifestLoaderError> {
        let manifest = LocalFileLoader::try_subgraph_dir(subgraph_path)?;
        Ok(Self(Arc::new(RwLock::new(manifest))))
    }

    pub fn abis(&self) -> ABIs {
        let manifest = self.0.read().unwrap();
        manifest.abis.clone()
    }

    pub fn schema(&self) -> SchemaLookup {
        let manifest = self.0.read().unwrap();
        manifest.schema.clone()
    }

    pub fn get_wasm(&self, source_name: &str) -> Vec<u8> {
        let manifest = self.0.read().unwrap();
        manifest.wasms.get(source_name).unwrap()
    }

    pub fn datasources(&self) -> DatasourceBundles {
        let manifest = self.0.read().unwrap();
        manifest.datasources.clone()
    }

    pub fn datasources_take_last(&self, last_n: usize) -> Vec<DatasourceBundle> {
        let manifest = self.0.read().unwrap();
        manifest.datasources.take_last(last_n)
    }

    pub fn count_datasources(&self) -> usize {
        let manifest = self.0.read().unwrap();
        manifest.datasources.len()
    }

    pub fn min_start_block(&self) -> u64 {
        let manifest = self.0.read().unwrap();
        manifest.subgraph_yaml.min_start_block()
    }

    pub fn datasource_and_templates(&self) -> DatasourceBundles {
        let manifest = self.0.read().unwrap();
        let mut active_ds = manifest.datasources.clone();
        let pending_ds = manifest.templates.clone();
        active_ds.extend(pending_ds);
        active_ds
    }

    pub fn create_datasource(
        &self,
        name: &str,
        params: Vec<String>,
        block_number: u64,
    ) -> Result<(), ManifestLoaderError> {
        let mut manifest = self.0.write().unwrap();
        let address = params.first().cloned();

        if address.is_none() {
            error!(
                ManifestAgent,
                "invalid datasource create, address must not be None"
            );
            return Err(ManifestLoaderError::CreateDatasourceFail);
        }

        let mut new_ds = manifest
            .templates
            .get(name, address.clone())
            .ok_or_else(|| {
                error!(
                    ManifestAgent,
                    format!("no template match datasource name={name}")
                );
                ManifestLoaderError::CreateDatasourceFail
            })?;

        new_ds.ds.source.address = address;
        new_ds.ds.source.startBlock = Some(block_number);

        manifest.datasources.add(new_ds).map_err(|e| {
            error!(ManifestAgent, format!("{:?}", e));
            ManifestLoaderError::CreateDatasourceFail
        })?;

        Ok(())
    }
}
