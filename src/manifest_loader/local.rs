use super::LoaderTrait;
use crate::common::*;
use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;

pub struct LocalFileLoader {
    pub subgraph_dir: String,
    pub subgraph_yaml: SubgraphYaml,
    pub abis: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[async_trait]
impl LoaderTrait for LocalFileLoader {
    async fn new(subgraph_dir: &str) -> Result<Self, ManifestLoaderError> {
        let md = fs::metadata(&subgraph_dir).unwrap();

        if !md.is_dir() {
            return Err(ManifestLoaderError::InvalidBuildDir(
                subgraph_dir.to_string(),
            ));
        }

        let mut this = Self {
            subgraph_dir: subgraph_dir.to_owned(),
            subgraph_yaml: SubgraphYaml::default(),
            abis: HashMap::new(),
        };

        this.load_yaml().await?;
        this.load_abis().await?;
        Ok(this)
    }

    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError> {
        let yaml_path = format!("{}/subgraph.yaml", self.subgraph_dir);
        let f = fs::File::open(&yaml_path)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path.to_owned()))?;
        let reader = BufReader::new(f);

        let subgraph_yaml: SubgraphYaml = serde_yaml::from_reader(reader)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path))?;

        self.subgraph_yaml = subgraph_yaml;
        Ok(())
    }

    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError> {
        let mut self_abis = HashMap::new();

        for datasource in self.subgraph_yaml.dataSources.iter() {
            let datasource_name = datasource.name.to_owned();
            let mut ds_abis = HashMap::new();

            for abi in datasource.mapping.abis.iter() {
                let abi_json =
                    format!("{}/build/{datasource_name}/{}", self.subgraph_dir, abi.file);
                let f = fs::File::open(&abi_json)
                    .map_err(|_| ManifestLoaderError::InvalidABI(abi_json.to_owned()))?;
                let reader = BufReader::new(f);
                let value = serde_json::from_reader(reader)
                    .map_err(|_| ManifestLoaderError::InvalidABI(abi_json))?;
                ds_abis.insert(abi.name.to_owned(), value);
            }

            self_abis.insert(datasource_name, ds_abis);
        }

        self.abis = self_abis;
        Ok(())
    }

    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        let datasource = self
            .subgraph_yaml
            .dataSources
            .iter()
            .find(|ds| &ds.name == datasource_name);

        if datasource.is_none() {
            return Err(ManifestLoaderError::InvalidDataSource(
                datasource_name.to_owned(),
            ));
        }

        let wasm_file = format!(
            "{}/build/{datasource_name}/{datasource_name}.wasm",
            self.subgraph_dir
        );
        let wasm_bytes = fs::read(wasm_file.to_owned())
            .map_err(|_| ManifestLoaderError::InvalidWASM(wasm_file))?;

        Ok(wasm_bytes)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use env_logger;

    #[tokio::test]
    async fn test_local_file_loader() {
        env_logger::try_init().unwrap_or_default();
        let loader = LocalFileLoader::new("../subgraph-testing/packages/v0_0_5")
            .await
            .unwrap();

        assert_eq!(loader.subgraph_yaml.dataSources.len(), 3);
        assert_eq!(loader.abis.keys().len(), 3);

        loader.load_wasm("TestTypes").await.unwrap();
        loader.load_wasm("TestStore").await.unwrap();
        loader.load_wasm("TestDataSource").await.unwrap();
    }
}
