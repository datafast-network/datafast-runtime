use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::config::Config;
use crate::config::TransformConfig;
use crate::database::DatabaseAgent;
use crate::errors::TransformError;
use crate::wasm_host::create_wasm_host;
use crate::wasm_host::AscHost;
use semver::Version;
use std::collections::HashMap;
use std::str::FromStr;
use wasmer::Function;
use wasmer::RuntimeError;
use wasmer::Value;

#[derive(Clone, Debug)]
pub struct TransformRequest {
    value: serde_json::Value,
    transform: TransformConfig,
}

pub struct TransformFunction {
    name: String,
    func: Function,
}

struct Transform {
    host: AscHost,
    funcs: HashMap<String, TransformFunction>,
}

impl Transform {
    pub fn new(host: AscHost, conf: &Config) -> Self {
        assert!(conf.transforms.is_some());

        let transforms = conf.transforms.as_ref().unwrap();

        let funcs = transforms
            .into_iter()
            .fold(HashMap::new(), |mut acc, (name, trans_conf)| {
                let func = host
                    .instance
                    .exports
                    .get_function(&trans_conf.func_name)
                    .expect(
                        format!(
                            "Function {} not found in wasm module",
                            &trans_conf.func_name
                        )
                        .as_str(),
                    )
                    .to_owned();
                acc.insert(
                    name.clone(),
                    TransformFunction {
                        name: trans_conf.func_name.clone(),
                        func,
                    },
                );
                acc
            });

        Transform { host, funcs }
    }

    fn transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        request: TransformRequest,
    ) -> Result<R, TransformError> {
        let func_name = request.transform.func_name.clone();
        let func = self
            .funcs
            .get(&func_name)
            .ok_or(TransformError::InvalidFunctionName(func_name))?;

        let mut json_data = request.value.clone();
        let asc_json = asc_new(&mut self.host, &mut json_data)?;
        let ptr = asc_json.wasm_ptr();
        let result = func
            .func
            .call(&mut self.host.store, &[Value::I32(ptr as i32)])?;
        let result_ptr = result
            .first()
            .ok_or(TransformError::TransformFail(RuntimeError::new(
                "Invalid pointer",
            )))?
            .unwrap_i32() as u32;
        let asc_ptr = AscPtr::<P>::new(result_ptr);
        let result = asc_get(&self.host, asc_ptr, 0)?;
        Ok(result)
    }
}

/// # TransformInstance
/// Transform multi datasource data to subgraph data
/// Setup transform instance with config
pub struct TransformInstance {
    transforms: HashMap<String, Transform>,
}

impl TransformInstance {
    pub fn new(conf: &Config, db: DatabaseAgent) -> Result<Self, TransformError> {
        assert!(conf.transforms.is_some());
        let mut transforms = HashMap::new();
        for trans_conf in conf.transforms.as_ref().unwrap().values() {
            let version = Version::new(0, 0, 5);
            let wasm_bytes = std::fs::read(&trans_conf.wasm_path)
                .expect(format!("Failed to read wasm file {}", &trans_conf.wasm_path).as_str());
            let host = create_wasm_host(version, wasm_bytes, db.clone())?;
            transforms.insert(trans_conf.func_name.clone(), Transform::new(host, conf));
        }
        Ok(TransformInstance { transforms })
    }
    pub fn transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        request: TransformRequest,
    ) -> Result<R, TransformError> {
        let transform = self
            .transforms
            .get_mut(&request.transform.func_name)
            .ok_or(TransformError::InvalidFunctionName(
                request.transform.func_name.clone(),
            ))?;
        transform.transform_data(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ethereum::block::AscEthereumBlock;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::wasm_host::test::get_subgraph_testing_resource;
    use crate::wasm_host::test::mock_wasm_host;
    use std::fs::File;

    pub fn mock_transform(conf: &Config) -> TransformInstance {
        let transforms = conf.transforms.as_ref().unwrap().iter().fold(
            HashMap::new(),
            |mut trans, (name, trans_conf)| {
                let (version, wasm_path) =
                    get_subgraph_testing_resource("0.0.5", &trans_conf.datasource);
                let host = mock_wasm_host(version, &wasm_path);
                trans.insert(trans_conf.func_name.clone(), Transform::new(host, conf));
                trans
            },
        );
        TransformInstance { transforms }
    }

    #[tokio::test]
    async fn test_transform_full_block() {
        env_logger::try_init().unwrap_or_default();
        let mut transforms = HashMap::new();
        let transform_block = TransformConfig {
            datasource: "Ingestor".to_string(),
            func_name: "transformEthereumBlock".to_string(),
            wasm_path: "test".to_string(),
            wasm_version: "5".to_string(),
        };
        transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let mut transform = mock_transform(&conf);
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
        let request = TransformRequest {
            value: ingestor_block.clone(),
            transform: transform_block,
        };
        let block = transform
            .transform_data::<AscEthereumBlock, EthereumBlockData>(request)
            .unwrap();
        assert_eq!(format!("{:?}", block.number), "10000000");
        //asert_eq all fields of block
    }
}
