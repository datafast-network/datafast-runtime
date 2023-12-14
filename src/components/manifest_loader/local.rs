use super::LoaderTrait;
use super::SchemaLookup;
use crate::common::*;
use crate::errors::ManifestLoaderError;
use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::io::BufReader;

#[derive(Default)]
pub struct LocalFileLoader {
    pub subgraph_dir: String,
    pub subgraph_yaml: SubgraphYaml,
    pub abis: ABIList,
    pub schema: SchemaLookup,
    wasm_per_source: HashMap<String, Vec<u8>>,
}

impl LocalFileLoader {
    pub fn new(subgraph_dir: &str) -> Result<Self, ManifestLoaderError> {
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
            wasm_per_source: HashMap::new(),
        };

        this.load_yaml()?;
        this.load_abis()?;
        this.load_schema()?;
        Ok(this)
    }

    fn load_schema(&mut self) -> Result<(), ManifestLoaderError> {
        let schema_path = format!("{}/schema.graphql", self.subgraph_dir);
        let schema =
            read_to_string(schema_path).map_err(|_| ManifestLoaderError::SchemaParsingError)?;
        self.schema = SchemaLookup::new_from_graphql_schema(&schema);
        Ok(())
    }

    fn load_yaml(&mut self) -> Result<(), ManifestLoaderError> {
        let yaml_path = format!("{}/subgraph.yaml", self.subgraph_dir);
        let f = fs::File::open(&yaml_path)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path.to_owned()))?;
        let reader = BufReader::new(f);

        let subgraph_yaml: SubgraphYaml = serde_yaml::from_reader(reader)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path))?;

        self.subgraph_yaml = subgraph_yaml;
        Ok(())
    }

    fn load_abis(&mut self) -> Result<(), ManifestLoaderError> {
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

    fn load_wasm(&mut self, datasource_name: &str) -> Result<Vec<u8>, ManifestLoaderError> {
        if let Some(wasm_bytes) = self.wasm_per_source.get(datasource_name) {
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
            .insert(datasource_name.to_string(), wasm_bytes.clone());
        Ok(wasm_bytes)
    }
}

impl LoaderTrait for LocalFileLoader {
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

    fn get_wasm(&self, source_name: &str) -> Vec<u8> {
        self.wasm_per_source
            .get(source_name)
            .expect("invalid source name")
            .to_vec()
    }

    fn create_datasource(
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
        self.load_abis()?;
        self.load_wasm(name)?;
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

    #[test]
    fn test_local_file_loader() {
        env_logger::try_init().unwrap_or_default();
        let mut loader = LocalFileLoader::new("../subgraph-testing/packages/v0_0_5/build").unwrap();

        assert_eq!(loader.subgraph_yaml.dataSources.len(), 5);
        assert_eq!(loader.abis.len(), 5);

        loader.load_wasm("TestTypes").unwrap();
        loader.load_wasm("TestStore").unwrap();
        loader.load_wasm("TestDataSource").unwrap();

        loader.load_abis().unwrap();
        loader.load_yaml().unwrap();
    }

    #[test]
    fn test_get_template() {
        env_logger::try_init().unwrap_or_default();
        let loader = LocalFileLoader::new("./subgraph").unwrap();
        let sources = loader.datasources_and_templates();
        let template = sources.iter().find(|s| s.name == "Pool").unwrap();
        log::log!(log::Level::Info, "{:?}", template);
        assert_eq!(3, sources.len());
    }
}
