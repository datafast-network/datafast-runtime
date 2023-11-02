use kanal::AsyncReceiver;
use kanal::SendError;
use kanal::Sender;
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
    input_receiver: AsyncReceiver<TransformRequest>,
    output_forwarder: Sender<SubgraphData>,
}

impl Transformer {
    fn handle_transform_request(
        &mut self,
        request: TransformRequest,
    ) -> Result<(), TransformerError> {
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
        self.output_forwarder.send(SubgraphData::Block(eth_block))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::host_fn_test;

    use super::*;

    #[test]
    fn test_transform_block_data() {
        // host_fn_test!("test")
    }
}
