use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::chain::ethereum::block::EthereumFullBlock;
use crate::config::Config;
use crate::config::TransformConfig;
use crate::transform::errors::TransformError;
use crate::wasm_host::AscHost;
use std::collections::HashMap;
use wasmer::Function;
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

pub struct Transform {
    host: AscHost,
    funcs: HashMap<String, TransformFunction>,
}

impl Transform {
    pub fn new(host: AscHost, conf: &Config) -> Result<Self, TransformError> {
        let mut funcs = HashMap::new();
        assert!(conf.transforms.is_some());
        let transforms = conf.transforms.as_ref().unwrap();
        for (name, transform) in transforms {
            let func = host
                .instance
                .exports
                .get_function(&transform.func_name)?
                .to_owned();
            funcs.insert(
                name.clone(),
                TransformFunction {
                    name: transform.func_name.clone(),
                    func,
                },
            );
        }
        Ok(Transform { host, funcs })
    }

    pub fn transform_data<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        request: &TransformRequest,
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

        let asc_ptr = AscPtr::<P>::new(result.first().unwrap().unwrap_i32() as u32);
        let result = asc_get(&self.host, asc_ptr, 0).expect("Failed to get result");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ethereum::block::AscEthereumFullBlock;
    use crate::wasm_host::test::get_subgraph_testing_resource;
    use crate::wasm_host::test::mock_wasm_host;
    use std::fs::File;

    #[tokio::test]
    async fn test_transform_full_block() {
        env_logger::try_init().unwrap_or_default();
        let mut transforms = HashMap::new();
        let transform_block = TransformConfig {
            datasource: "TestTypes".to_string(),
            func_name: "transformFullBlock".to_string(),
        };
        transforms.insert(transform_block.func_name.clone(), transform_block.clone());
        let conf = Config {
            subgraph_name: "".to_string(),
            subgraph_id: None,
            manifest: "".to_string(),
            transforms: Some(transforms),
        };
        let (version, wasm_path) = get_subgraph_testing_resource("0.0.5", "TestTypes");
        let host = mock_wasm_host(version, &wasm_path);
        let mut transform = Transform::new(host, &conf).unwrap();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block: serde_json::Value = serde_json::from_reader(file_json).unwrap();
        let request = TransformRequest {
            value: ingestor_block.clone(),
            transform: transform_block,
        };
        let block: EthereumFullBlock = transform
            .transform_data::<AscEthereumFullBlock, _>(&request)
            .unwrap();
        assert_eq!(format!("{:?}", block.header.gas_limit), "9990236");
        assert_eq!(format!("{:?}", block.number), "10000000");
        //asert_eq all fields of block
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.logs.len(), 2);
        let transaction = block.transactions.get(0).unwrap();
        assert_eq!(
            format!("{:?}", transaction.hash),
            "0x4a1e3e3a2aa4aa79a777d0ae3e2c3a6de158226134123f6c14334964c6ec70cf"
        );
        assert_eq!(format!("{:?}", transaction.nonce), "25936206");
        let log = block.logs.get(0).unwrap();
        assert_eq!(
            format!("{:?}", log.address),
            "0xced4e93198734ddaff8492d525bd258d49eb388e"
        );
        assert_eq!(log.block_number.unwrap(), block.number)
    }
}
