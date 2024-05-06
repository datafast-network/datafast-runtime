mod local;

use crate::common::*;
use crate::error;
use crate::errors::ManifestLoaderError;
use local::LocalFileLoader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct ManifestBundle {
    subgraph_yaml: SubgraphYaml,
    templates: DatasourceBundles,
    abis: ABIs,
    wasms: WASMs,
    schema: Schemas,
    datasources: DatasourceBundles,
    block_ptr: BlockPtr,
    templates_address_filter: HashMap<String, HashSet<String>>,
}

#[derive(Clone, Default)]
pub struct ManifestAgent(Rc<RefCell<ManifestBundle>>);

unsafe impl Send for ManifestAgent {}

impl ManifestAgent {
    pub async fn new(subgraph_path: &str) -> Result<Self, ManifestLoaderError> {
        let manifest = LocalFileLoader::try_subgraph_dir(subgraph_path)?;
        Ok(Self(Rc::new(RefCell::new(manifest))))
    }

    pub fn set_block_ptr(&self, block_ptr: &BlockPtr) {
        let mut manifest = self.0.borrow_mut();
        manifest.block_ptr = block_ptr.clone();
    }

    pub fn abis(&self) -> ABIs {
        let manifest = self.0.borrow();
        manifest.abis.clone()
    }

    pub fn schemas(&self) -> Schemas {
        let manifest = self.0.borrow();
        manifest.schema.clone()
    }

    pub fn get_wasm(&self, source_name: &str) -> Vec<u8> {
        let manifest = self.0.borrow();
        manifest.wasms.get(source_name).unwrap()
    }

    pub fn get_datasource(&self, name: &str) -> DatasourceBundle {
        let manifest = self.0.borrow();
        let bundle_from_datasource = manifest.datasources.ds.iter().find(|ds| ds.name() == name);

        if let Some(source) = bundle_from_datasource {
            return source.clone();
        }

        let bundle_from_template = manifest.templates.ds.iter().find(|ds| ds.name() == name);

        if let Some(source) = bundle_from_template {
            return source.clone();
        }

        panic!("bad datasource name {name}");
    }

    pub fn datasources(&self) -> DatasourceBundles {
        let manifest = self.0.borrow();
        manifest.datasources.clone()
    }

    pub fn datasources_take_from(&self, last_n: usize) -> Vec<DatasourceBundle> {
        let manifest = self.0.borrow();
        manifest.datasources.take_from(last_n)
    }

    pub fn count_datasources(&self) -> usize {
        let manifest = self.0.borrow();
        manifest.datasources.len()
    }

    pub fn min_start_block(&self) -> StartBlock {
        let manifest = self.0.borrow();
        manifest.subgraph_yaml.min_start_block().into()
    }

    pub fn datasource_and_templates(&self) -> DatasourceBundles {
        let manifest = self.0.borrow();
        let mut active_ds = manifest.datasources.clone();
        let pending_ds = manifest.templates.clone();
        active_ds.extend(pending_ds);
        active_ds
    }

    pub fn create_datasource(
        &self,
        name: &str,
        params: Vec<String>,
    ) -> Result<(), ManifestLoaderError> {
        let mut manifest = self.0.borrow_mut();
        let address = params.first().cloned().map(|s| s.to_lowercase());

        if address.is_none() {
            error!(
                Manifest,
                "invalid datasource create, address must not be None"
            );
            return Err(ManifestLoaderError::CreateDatasourceFail);
        }

        if !manifest.templates_address_filter.contains_key(name) {
            manifest
                .templates_address_filter
                .insert(name.to_string(), HashSet::new());
        }

        manifest
            .templates_address_filter
            .get_mut(name)
            .unwrap()
            .insert(address.clone().unwrap());

        Ok(())
    }

    pub fn should_process_address(&self, name: &str, address: &str) -> bool {
        let manifest = self.0.borrow();
        let template = manifest.templates_address_filter.get(name);

        if let Some(source_address) = template {
            return source_address.contains(address);
        }

        true
    }
}
