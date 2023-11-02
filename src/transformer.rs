use kanal::SendError;
use std::collections::HashMap;
use thiserror::Error;
use wasmer::Function;
use wasmer::RuntimeError;
use wasmer::Value;

use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::errors::AscError;
use crate::chain::ethereum::block::AscEthereumBlock;
use crate::chain::ethereum::block::EthereumBlockData;
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

pub struct TransformFunction {
    name: String,
    func: Function,
}

pub struct Transformer {
    host: AscHost,
    funcs: HashMap<String, TransformFunction>,
}

impl Transformer {
    fn handle_transform_request(
        &mut self,
        request: TransformRequest,
    ) -> Result<SubgraphData, TransformerError> {
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

        let asc_block =
            AscPtr::<AscEthereumBlock>::new(result.first().unwrap().unwrap_i32() as u32);
        let eth_block: EthereumBlockData = asc_get(&self.host, asc_block, 0).unwrap();
        Ok(SubgraphData::Block(eth_block))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::wasm_host::test::get_subgraph_testing_resource;
    use crate::wasm_host::test::mock_wasm_host;
    use kanal;
    use serde_json::json;
    use tokio::join;

    #[tokio::test]
    async fn test_transformer() {
        use env_logger;
        use std::env;

        env::set_var("SUBGRAPH_WASM_RUNTIME_TEST", "YES");
        env_logger::try_init().unwrap_or_default();

        let (s1, r1) = kanal::bounded_async(1);
        let (s2, r2) = kanal::bounded_async(1);

        let (version, wasm_path) = get_subgraph_testing_resource("0.0.5", "TestTransform");
        let host = mock_wasm_host(version, &wasm_path);

        let mut transformer = Transformer {
            host,
            funcs: HashMap::new(),
        };

        let t1 = async move {
            while let Ok(request) = r1.recv().await {
                let result = transformer.handle_transform_request(request).unwrap();
                s2.send(result).await.unwrap();
            }
        };

        let t2 = async move {
            while let Ok(data) = r2.recv().await {
                ::log::info!("Received transformed block: {:?}", data);
            }
        };

        let ingestor_block = json!({});
        let request = TransformRequest {
            value: ingestor_block,
            transform: Transform {
                datasource: "TestTransform".to_string(),
                func_name: "transformBlock".to_string(),
            },
        };

        let send_request = s1.send(request);

        let _result = join!(t1, t2, send_request);
    }
}
