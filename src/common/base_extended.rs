use super::base::*;
use ethabi::Contract;
use semver::Version;
use std::collections::HashMap;
use std::fmt::Display;

impl SubgraphYaml {
    pub fn abis(&self) -> HashMap<String, String> {
        let mut abis = HashMap::new();
        for ds in self.dataSources.iter() {
            for mapping_abi in ds.mapping.abis.iter() {
                abis.insert(mapping_abi.name.clone(), mapping_abi.file.clone());
            }
        }
        for ds in self.templates.clone().unwrap_or(vec![]).iter() {
            for mapping_abi in ds.mapping.abis.iter() {
                abis.insert(mapping_abi.name.clone(), mapping_abi.file.clone());
            }
        }
        abis
    }

    pub fn wasms(&self) -> HashMap<String, String> {
        let mut wasms = HashMap::new();
        for ds in self.dataSources.iter() {
            wasms.insert(ds.name.clone(), ds.mapping.file.clone());
        }
        for ds in self.templates.clone().unwrap_or(vec![]) {
            wasms.insert(ds.name, ds.mapping.file);
        }
        wasms
    }

    pub fn min_start_block(&self) -> u64 {
        self.dataSources
            .iter()
            .map(|ds| ds.source.startBlock.unwrap_or(0))
            .min()
            .unwrap_or(0)
    }
}

impl BlockPtr {
    pub fn is_parent(&self, child_block_ptr: &BlockPtr) -> bool {
        self.number == child_block_ptr.number - 1 && self.hash == child_block_ptr.parent_hash
    }
}

impl Display for BlockPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BlockPtr({}, hash=`{}`, parent_hash=`{}`)",
            self.number, self.hash, self.parent_hash
        )
    }
}

impl FromIterator<(String, serde_json::Value)> for ABIs {
    fn from_iter<I: IntoIterator<Item = (String, serde_json::Value)>>(iter: I) -> Self {
        Self(iter.into_iter().collect::<HashMap<_, _>>())
    }
}

impl ABIs {
    #[cfg(test)]
    pub fn names(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<serde_json::Value> {
        self.0.get(name).cloned()
    }

    pub fn get_contract(&self, name: &str) -> Option<Contract> {
        self.0
            .get(name)
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn insert(&mut self, name: String, abi: serde_json::Value) {
        self.0.insert(name, abi);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl FromIterator<(String, Vec<u8>)> for WASMs {
    fn from_iter<I: IntoIterator<Item = (String, Vec<u8>)>>(iter: I) -> Self {
        Self(iter.into_iter().collect::<HashMap<_, _>>())
    }
}

impl WASMs {
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.0.get(name).cloned()
    }

    pub fn insert(&mut self, name: String, wasm: Vec<u8>) {
        self.0.insert(name, wasm);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<&DatasourceBundle> for Datasource {
    fn from(source: &DatasourceBundle) -> Self {
        source.ds.clone()
    }
}

impl DatasourceBundle {
    pub fn api_version(&self) -> Version {
        self.ds.mapping.apiVersion.clone()
    }

    pub fn network(&self) -> String {
        self.ds.network.clone()
    }

    pub fn address(&self) -> Option<String> {
        self.ds.source.address.clone().map(|s| s.to_lowercase())
    }

    pub fn wasm(&self) -> Vec<u8> {
        self.wasm.clone()
    }

    pub fn name(&self) -> String {
        self.ds.name.clone()
    }

    pub fn start_block(&self) -> u64 {
        self.ds.source.startBlock.unwrap_or(0)
    }
}

impl DatasourceBundles {
    pub fn inner(self) -> Vec<DatasourceBundle> {
        self.ds
    }

    pub fn len(&self) -> usize {
        self.ds.len()
    }

    pub fn get(&self, name: &str, address: Option<String>) -> Option<DatasourceBundle> {
        self.ds
            .iter()
            .find(|ds| ds.name() == name && ds.address() == address)
            .cloned()
            .clone()
    }

    pub fn add(&mut self, ds: DatasourceBundle) -> Result<(), String> {
        if !self.keys.insert((ds.name(), ds.address())) {
            return Err("Datasource already exist".to_owned());
        }

        self.ds.push(ds);
        Ok(())
    }

    pub fn extend(&mut self, ds: DatasourceBundles) {
        for ds in ds.iter() {
            self.add(ds.clone()).ok();
        }
    }

    pub fn iter(&self) -> Vec<&DatasourceBundle> {
        self.ds.iter().collect()
    }

    pub fn take_from(&self, last_n: usize) -> Vec<DatasourceBundle> {
        self.ds[last_n..].to_vec()
    }
}

impl From<(&Vec<Datasource>, &ABIs, &WASMs)> for DatasourceBundles {
    fn from((sources, abis, wasms): (&Vec<Datasource>, &ABIs, &WASMs)) -> Self {
        let ds = sources
            .iter()
            .map(|ds| DatasourceBundle {
                ds: ds.clone(),
                abi: abis.get(&ds.source.abi).unwrap(),
                wasm: wasms.get(&ds.name).unwrap(),
            })
            .collect::<Vec<DatasourceBundle>>();
        let keys = ds.iter().map(|dsb| (dsb.name(), dsb.address())).collect();
        Self { ds, keys }
    }
}

impl From<DatasourceBundles> for Vec<Datasource> {
    fn from(bundles: DatasourceBundles) -> Self {
        bundles.ds.into_iter().map(|ds| ds.ds).collect()
    }
}
