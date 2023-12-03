use super::LoaderTrait;
use super::SchemaLookup;
use crate::common::*;
use crate::errors::ManifestLoaderError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::io::BufReader;
use tokio::sync::Mutex;

pub struct LocalFileLoader {
    pub subgraph_dir: String,
    pub subgraph_yaml: SubgraphYaml,
    pub abis: HashMap<String, serde_json::Value>,
    pub schema: SchemaLookup,
    wasm_per_source: Mutex<HashMap<String, Vec<u8>>>,
}

impl LocalFileLoader {
    pub async fn new(subgraph_dir: &str) -> Result<Self, ManifestLoaderError> {
        let md = fs::metadata(subgraph_dir)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphDir(subgraph_dir.to_string()))?;

        if !md.is_dir() {
            return Err(ManifestLoaderError::InvalidBuildDir(
                subgraph_dir.to_string(),
            ));
        }

        let mut this = Self {
            subgraph_dir: subgraph_dir.to_owned(),
            subgraph_yaml: SubgraphYaml::default(),
            abis: HashMap::new(),
            schema: SchemaLookup::new(),
            wasm_per_source: Mutex::new(HashMap::new()),
        };

        this.load_yaml().await?;
        this.load_abis().await?;
        this.load_schema().await?;
        Ok(this)
    }
}

#[async_trait]
impl LoaderTrait for LocalFileLoader {
    async fn load_schema(&mut self) -> Result<(), ManifestLoaderError> {
        let schema_path = format!("{}/schema.graphql", self.subgraph_dir);
        let schema =
            read_to_string(schema_path).map_err(|_| ManifestLoaderError::SchemaParsingError)?;
        self.schema = SchemaLookup::new_from_graphql_schema(&schema);
        Ok(())
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
        for datasource in self.subgraph_yaml.dataSources.iter_mut() {
            let datasource_name = datasource.name.to_owned();
            let abi_name = datasource.source.abi.clone();
            let abi_path = datasource
                .mapping
                .abis
                .iter()
                .find(|abi| abi.name == abi_name)
                .map_or(
                    Err(ManifestLoaderError::InvalidABI(abi_name.to_owned())),
                    |abi| Ok(format!("{}/{}", self.subgraph_dir, abi.file)),
                )?;
            let abi_file = fs::File::open(&abi_path)
                .map_err(|_| ManifestLoaderError::InvalidABI(abi_path.to_owned()))?;
            let value = serde_json::from_reader(abi_file)
                .map_err(|_| ManifestLoaderError::InvalidABI(abi_path.to_owned()))?;
            datasource.source.abi = serde_json::to_string(&value).unwrap();
            self.abis.insert(datasource_name, value);
        }
        Ok(())
    }

    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        if let Some(wasm_bytes) = self.wasm_per_source.lock().await.get(datasource_name) {
            return Ok(wasm_bytes.clone());
        }

        let datasource = self
            .subgraph_yaml
            .dataSources
            .iter()
            .find(|ds| ds.name == datasource_name);

        if datasource.is_none() {
            return Err(ManifestLoaderError::InvalidDataSource(
                datasource_name.to_owned(),
            ));
        }

        let file_path = datasource.unwrap().mapping.file.clone();
        let wasm_file = format!("{}/{file_path}", self.subgraph_dir);

        let wasm_bytes =
            fs::read(&wasm_file).map_err(|_| ManifestLoaderError::InvalidWASM(wasm_file))?;

        self.wasm_per_source
            .lock()
            .await
            .insert(datasource_name.to_string(), wasm_bytes.clone());
        Ok(wasm_bytes)
    }

    fn get_abis(&self) -> HashMap<String, serde_json::Value> {
        self.abis.clone()
    }

    fn get_schema(&self) -> SchemaLookup {
        self.schema.to_owned()
    }

    fn get_sources(&self) -> Vec<Source> {
        self.subgraph_yaml
            .dataSources
            .iter()
            .map(|ds| ds.source.clone())
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_local_file_loader() {
        env_logger::try_init().unwrap_or_default();
        let mut loader = LocalFileLoader::new("../subgraph-testing/packages/v0_0_5/build")
            .await
            .unwrap();

        assert_eq!(loader.subgraph_yaml.dataSources.len(), 5);
        assert_eq!(loader.abis.keys().len(), 5);

        loader.load_wasm("TestTypes").await.unwrap();
        loader.load_wasm("TestStore").await.unwrap();
        loader.load_wasm("TestDataSource").await.unwrap();

        loader.load_abis().await.unwrap();
        loader.load_yaml().await.unwrap();
    }
}
