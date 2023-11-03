use kanal::SendError;
use std::collections::HashMap;
use thiserror::Error;
use wasmer::Function;
use wasmer::RuntimeError;
use wasmer::Value;

use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::asc::errors::AscError;
use crate::chain::ethereum::block::AscEthereumBlock;
use crate::config::Transform;
use crate::messages::SubgraphData;
use crate::wasm_host::*;

#[derive(Debug, Error)]
pub enum TransformerError {
    #[error("No transformer function with name={0}")]
    InvalidFunctionName(String),
    #[error("Failed to allocate memory for input data")]
    InputAllocationFail(#[from] AscError),
    #[error("Transfor failed: {0}")]
    TransfomFail(#[from] RuntimeError),
    #[error("Forwarding data fail")]
    ForwardDataFail(#[from] SendError),
}

pub struct TransformRequest {
    value: serde_json::Value,
    transform: Transform,
}

impl TransformRequest {
    pub fn new(value: serde_json::Value, transform: Transform) -> Self {
        TransformRequest { value, transform }
    }
}

pub struct TransformFunction {
    name: String,
    func: Function,
}

pub struct Transformer {
    host: AscHost,
    funcs: HashMap<String, TransformFunction>,
}

impl Transformer {
    fn handle_transform_request<P: AscType + AscIndexId, R: FromAscObj<P>>(
        &mut self,
        request: TransformRequest,
    ) -> Result<R, TransformerError> {
        let func_name = request.transform.func_name;
        let func = self
            .funcs
            .get(&func_name)
            .ok_or(TransformerError::InvalidFunctionName(func_name))?;

        let mut json_data = request.value;
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
mod test {
    use super::*;
    use crate::chain::ethereum::block::AscFullBlock;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::chain::ethereum::block::EthereumFullBlock;
    use crate::wasm_host::test::get_subgraph_testing_resource;
    use crate::wasm_host::test::mock_wasm_host;
    use env_logger;
    use kanal;
    use serde_json::json;
    use std::fs::File;
    use tokio::join;

    #[tokio::test]
    async fn test_transformer() {
        env_logger::try_init().unwrap_or_default();

        let (s1, r1) = kanal::bounded_async(1);
        let (s2, r2) = kanal::bounded_async(1);
        let (version, wasm_path) = get_subgraph_testing_resource("0.0.5", "TestTypes");
        let host = mock_wasm_host(version, &wasm_path);

        let transform_block_function = host
            .instance
            .exports
            .get_function("transformEthereumBlock")
            .unwrap()
            .to_owned();
        let mut funcs = HashMap::new();
        funcs.insert(
            "transformEthereumBlock".to_string(),
            TransformFunction {
                name: "transformEthereumBlock".to_string(),
                func: transform_block_function,
            },
        );
        let mut transformer = Transformer { host, funcs };

        // Transformer listening for incoming data
        let t1 = async move {
            while let Ok(request) = r1.recv().await {
                let result = transformer
                    .handle_transform_request::<AscEthereumBlock, _>(request)
                    .unwrap();
                s2.send(SubgraphData::Block(result)).await.unwrap();
                return;
            }
        };

        // Collecting result from transformer
        let t2 = async move {
            while let Ok(SubgraphData::Block(block)) = r2.recv().await {
                ::log::info!("Transformed data: \n{:?}\n", block);
                assert_eq!(block.number.to_string(), "10000000");
                assert_eq!(
                    format!("{:?}", block.hash),
                    "0xaa20f7bde5be60603f11a45fc4923aab7552be775403fc00c2e6b805e6297dbe"
                );
                return;
            }
            panic!("test failed");
        };
        let start = std::time::Instant::now();
        // let file_json = File::open("/Users/quannguyen/block_10000000_safe_size.json").unwrap();
        let file_json = File::open("./block.json").unwrap();
        // Send test data for transform
        let ingestor_block = serde_json::from_reader(file_json).unwrap();
        ::log::info!("Input data success {:?}", start.elapsed());

        let request = TransformRequest {
            value: ingestor_block,
            transform: Transform {
                datasource: "TestTypes".to_string(),
                func_name: "transformEthereumBlock".to_string(),
            },
        };

        // Collecting the threads
        let _result = join!(t1, t2, s1.send(request));
    }
}
