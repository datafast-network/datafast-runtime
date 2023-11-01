use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use serde_yaml;
use std::collections::HashMap;
use std::env::current_dir;
use std::fmt::Debug;
use std::fs;
use std::io::BufReader;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct MappingABI {
    name: String,
    file: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct Mapping {
    kind: String,
    apiVersion: Version,
    entities: Vec<String>,
    abis: Vec<MappingABI>,
    eventHandlers: Option<Vec<HashMap<String, String>>>,
    blockHandlers: Option<Vec<HashMap<String, String>>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Datasource {
    kind: String,
    name: String,
    network: String,
    source: HashMap<String, String>,
    mapping: Mapping,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
struct SubgraphYaml {
    dataSources: Vec<Datasource>,
}

#[async_trait]
pub trait LoaderTrait: Sized {
    async fn new(path: &str) -> Result<Self, ManifestLoaderError>;
    async fn load_yaml(&mut self) -> Result<(), ManifestLoaderError>;
    async fn load_abis(&mut self) -> Result<(), ManifestLoaderError>;
    // Load-Wasm is lazy, we only execute it when we need it
    async fn load_wasm(&self) -> Result<HashMap<String, Vec<u8>>, ManifestLoaderError>;
}

pub struct LocalFileLoader {
    subgraph_dir: String,
    subgraph_yaml: SubgraphYaml,
    abis: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[async_trait]
impl LoaderTrait for LocalFileLoader {
    async fn new(relative_subgraph_dir_path: &str) -> Result<Self, ManifestLoaderError> {
        let mut current_path = current_dir().unwrap();
        current_path.push(relative_subgraph_dir_path);
        let subgraph_dir = current_path.into_os_string().into_string().unwrap();
        let md = fs::metadata(&subgraph_dir).unwrap();

        if !md.is_dir() {
            return Err(ManifestLoaderError::InvalidBuildDir(
                relative_subgraph_dir_path.to_string(),
            ));
        }

        let mut this = Self {
            subgraph_dir,
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

    async fn load_wasm(&self) -> Result<HashMap<String, Vec<u8>>, ManifestLoaderError> {
        let mut wasm_files = HashMap::new();

        for datasource in self.subgraph_yaml.dataSources.iter() {
            let datasource_name = datasource.name.to_owned();
            let wasm_file = format!(
                "{}/build/{datasource_name}/{datasource_name}.wasm",
                self.subgraph_dir
            );
            let wasm_bytes = fs::read(wasm_file.to_owned())
                .map_err(|_| ManifestLoaderError::InvalidWASM(wasm_file))?;
            wasm_files.insert(datasource_name, wasm_bytes);
        }

        Ok(wasm_files)
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

        let wasm_files = loader.load_wasm().await.unwrap();
        assert_eq!(wasm_files.len(), 3);
    }
}
