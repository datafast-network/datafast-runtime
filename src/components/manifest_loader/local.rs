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

#[derive(Default)]
pub struct LocalFileLoader {
    pub subgraph_dir: String,
    pub subgraph_yaml: SubgraphYaml,
    pub abis: ABIList,
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
            abis: ABIList::default(),
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
        let ds = self.datasources_and_templates();
        for datasource in ds {
            for mapping_abi in datasource.mapping.abis.iter() {
                let abi_name = &mapping_abi.name;
                let abi_path = format!("{}/{}", self.subgraph_dir, mapping_abi.file);
                let abi_file = fs::File::open(&abi_path)
                    .map_err(|_| ManifestLoaderError::InvalidABI(abi_path.to_owned()))?;
                let value = serde_json::from_reader(abi_file)
                    .map_err(|_| ManifestLoaderError::InvalidABI(abi_path.to_owned()))?;
                self.abis.insert(abi_name.to_owned(), value);
            }
        }
        Ok(())
    }

    async fn load_wasm(&self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        if let Some(wasm_bytes) = self.wasm_per_source.lock().await.get(datasource_name) {
            return Ok(wasm_bytes.clone());
        }

        let datasources = self.datasources_and_templates();

        let datasource = datasources.iter().find(|ds| ds.name == datasource_name);
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

    fn get_abis(&self) -> ABIList {
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
    async fn create_datasource(
        &mut self,
        name: &str,
        params: Vec<String>,
        block_ptr: BlockPtr,
    ) -> Result<(), ManifestLoaderError> {
        let mut template = self
            .subgraph_yaml
            .templates
            .iter()
            .find(|t| t.name == name)
            .ok_or(ManifestLoaderError::InvalidDataSource(name.to_owned()))
            .cloned()?;
        template.source = Source {
            abi: template.source.abi,
            address: params.get(0).cloned(),
            startBlock: Some(block_ptr.number),
        };
        self.subgraph_yaml.dataSources.push(template);
        self.load_abis().await?;
        self.load_wasm(name).await?;
        Ok(())
    }
    fn datasources_and_templates(&self) -> Vec<Datasource> {
        let mut datasources = self.subgraph_yaml.dataSources.clone();
        datasources.extend(self.subgraph_yaml.templates.clone());
        datasources
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
        // assert_eq!(loader.abis, 5);

        loader.load_wasm("TestTypes").await.unwrap();
        loader.load_wasm("TestStore").await.unwrap();
        loader.load_wasm("TestDataSource").await.unwrap();

        loader.load_abis().await.unwrap();
        loader.load_yaml().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_template() {
        env_logger::try_init().unwrap_or_default();
        let loader = LocalFileLoader::new("./subgraph").await.unwrap();
        let sources = loader.datasources_and_templates();
        let template = sources.iter().find(|s| s.name == "Pool").unwrap();
        log::log!(log::Level::Info, "{:?}", template);
        assert_eq!(3, sources.len());
    }
}
