use super::ManifestBundle;
use super::SchemaLookup;
use crate::common::*;
use crate::errors::ManifestLoaderError;
use std::fs;
use std::fs::read_to_string;
use std::io::BufReader;

#[derive(Default)]
pub struct LocalFileLoader;

impl LocalFileLoader {
    pub fn try_subgraph_dir(subgraph_dir: &str) -> Result<ManifestBundle, ManifestLoaderError> {
        let md = fs::metadata(subgraph_dir)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphDir(subgraph_dir.to_string()))?;

        if !md.is_dir() {
            return Err(ManifestLoaderError::InvalidBuildDir(
                subgraph_dir.to_string(),
            ));
        }

        let subgraph_yaml = LocalFileLoader::load_yaml(subgraph_dir)?;
        let abis = LocalFileLoader::load_abis(subgraph_dir, &subgraph_yaml)?;
        let wasms = LocalFileLoader::load_wasm(subgraph_dir, &subgraph_yaml)?;
        let schema = LocalFileLoader::load_schema(subgraph_dir)?;
        let datasources = DatasourceBundles::from((&subgraph_yaml.dataSources, &abis, &wasms));
        let templates = DatasourceBundles::from((
            subgraph_yaml.templates.clone().unwrap_or(vec![]).as_ref(),
            &abis,
            &wasms,
        ));

        let manifest = ManifestBundle {
            subgraph_yaml,
            abis,
            wasms,
            schema,
            datasources,
            templates,
        };

        Ok(manifest)
    }

    fn load_schema(subgraph_dir: &str) -> Result<SchemaLookup, ManifestLoaderError> {
        let schema_path = format!("{}/schema.graphql", subgraph_dir);
        let schema =
            read_to_string(schema_path).map_err(|_| ManifestLoaderError::SchemaParsingError)?;
        Ok(SchemaLookup::new_from_graphql_schema(&schema))
    }

    fn load_yaml(subgraph_dir: &str) -> Result<SubgraphYaml, ManifestLoaderError> {
        let yaml_path = format!("{}/subgraph.yaml", subgraph_dir);
        let f = fs::File::open(&yaml_path)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path.to_owned()))?;
        let reader = BufReader::new(f);

        let subgraph_yaml: SubgraphYaml = serde_yaml::from_reader(reader)
            .map_err(|_| ManifestLoaderError::InvalidSubgraphYAML(yaml_path))?;
        Ok(subgraph_yaml)
    }

    fn load_abis(
        subgraph_dir: &str,
        subgraph_yaml: &SubgraphYaml,
    ) -> Result<ABIs, ManifestLoaderError> {
        let raw_abis = subgraph_yaml.abis();
        let abis = raw_abis
            .into_iter()
            .map(|(name, file_path)| {
                let abi_path = format!("{}/{}", subgraph_dir, file_path);
                let abi_file = fs::File::open(abi_path).unwrap();
                let value = serde_json::from_reader(abi_file).unwrap();
                (name, value)
            })
            .collect();
        Ok(abis)
    }

    fn load_wasm(
        subgraph_dir: &str,
        subgraph_yaml: &SubgraphYaml,
    ) -> Result<WASMs, ManifestLoaderError> {
        let wasms = subgraph_yaml.wasms();
        let wasms = wasms
            .into_iter()
            .map(|(datasource_name, wasm_file)| {
                let wasm_file = format!("{subgraph_dir}/{wasm_file}");
                let wasm_bytes = fs::read(wasm_file).unwrap();
                (datasource_name, wasm_bytes)
            })
            .collect();
        Ok(wasms)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::components::ManifestAgent;

    #[test]
    fn test_local_file_loader() {
        env_logger::try_init().unwrap_or_default();
        let m =
            LocalFileLoader::try_subgraph_dir("../subgraph-testing/packages/v0_0_5/build").unwrap();

        assert_eq!(
            vec![
                "ERC20NameBytes",
                "ERC20SymbolBytes",
                "ERC20",
                "Pool",
                "Factory"
            ]
            .sort(),
            m.abis.names().sort()
        );

        assert_eq!(m.subgraph_yaml.dataSources.len(), 6);
        assert!(m.subgraph_yaml.templates.is_none());
    }

    #[tokio::test]
    async fn test_get_template() {
        env_logger::try_init().unwrap_or_default();
        let m = ManifestAgent::new("../subgraph-testing/packages/uniswap-v3/build")
            .await
            .unwrap();
        assert_eq!(m.abis().len(), 6);
        assert_eq!(m.schema().get_entity_names().len(), 20);
        assert_eq!(m.datasources().len(), 2);
        assert_eq!(m.datasource_and_templates().len(), 3);
    }
}
