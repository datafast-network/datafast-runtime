mod ethereum;
mod types;

use crate::common::ABIs;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::config::Config;
use crate::errors::RPCError;
use crate::warn;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
pub use types::*;

#[async_trait]
pub trait RPCTrait {
    async fn handle_request(&mut self, call: CallRequestContext) -> Result<CallResponse, RPCError>;
    async fn get_latest_block(&mut self) -> Result<BlockPtr, RPCError>;
    fn cache_get(&self, call: &CallRequest) -> Option<CallResponse>;
    fn cache_set(&mut self, call: &CallRequest, result: &CallResponse) -> ();
}

pub enum RPCChain {
    None,
    Ethereum(ethereum::EthereumRPC),
}

#[async_trait]
impl RPCTrait for RPCChain {
    async fn handle_request(
        &mut self,
        request: CallRequestContext,
    ) -> Result<CallResponse, RPCError> {
        match self {
            RPCChain::Ethereum(client) => client.handle_request(request).await,
            RPCChain::None => Err(RPCError::InvalidChain),
        }
    }

    async fn get_latest_block(&mut self) -> Result<BlockPtr, RPCError> {
        match self {
            RPCChain::Ethereum(client) => client.get_latest_block().await,
            RPCChain::None => Ok(BlockPtr::default()),
        }
    }

    fn cache_get(&self, call: &CallRequest) -> Option<CallResponse> {
        match self {
            RPCChain::Ethereum(client) => client.cache_get(call),
            RPCChain::None => None,
        }
    }

    fn cache_set(&mut self, call: &CallRequest, result: &CallResponse) -> () {
        match self {
            RPCChain::Ethereum(client) => client.cache_set(call, result),
            RPCChain::None => (),
        }
    }
}

pub struct RpcClient {
    rpc_client: RPCChain,
    block_ptr: BlockPtr,
}

impl RpcClient {
    async fn new(config: &Config, abis: ABIs) -> Result<Self, RPCError> {
        let rpc_client = match config.chain {
            Chain::Ethereum => {
                let client = ethereum::EthereumRPC::new(&config.rpc_endpoint, abis).await?;
                RPCChain::Ethereum(client)
            }
        };
        Ok(Self {
            rpc_client,
            block_ptr: BlockPtr::default(),
        })
    }

    pub async fn handle_request(&mut self, call: CallRequest) -> Result<CallResponse, RPCError> {
        if let Some(result) = self.rpc_client.cache_get(&call) {
            return Ok(result);
        }

        let is_cachable = call.is_cachable();

        let call_context = CallRequestContext {
            block_ptr: self.block_ptr.clone(),
            call_request: call.clone(),
        };

        let result = self.rpc_client.handle_request(call_context).await?;

        if is_cachable {
            self.rpc_client.cache_set(&call, &result);
        }

        Ok(result)
    }

    pub fn new_mock() -> Self {
        Self {
            rpc_client: RPCChain::None,
            block_ptr: BlockPtr::default(),
        }
    }

    pub fn clear_cache(&mut self) {
        todo!()
    }

    pub fn set_block_ptr(&mut self, block_ptr: &BlockPtr) {
        self.block_ptr = block_ptr.clone();
    }
}

#[derive(Clone)]
pub struct RpcAgent(Arc<Mutex<RpcClient>>);

impl RpcAgent {
    pub async fn new(config: &Config, abis: ABIs) -> Result<Self, RPCError> {
        let rpc_client = RpcClient::new(config, abis).await?;
        Ok(Self(Arc::new(Mutex::new(rpc_client))))
    }

    pub fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCError> {
        let timer = Instant::now();
        use std::thread;
        let client = self.0.clone();

        let result = thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut rpc_agent = client.lock().await;
                rpc_agent.handle_request(call).await
            })
        })
        .join()
        .unwrap();

        warn!(RpcClient, "json-rpc call"; time => format!("{:?}ms", timer.elapsed().as_millis()));
        result
    }

    pub async fn set_block_ptr(&mut self, block_ptr: &BlockPtr) {
        let mut rpc = self.0.lock().await;
        rpc.set_block_ptr(block_ptr);
    }

    pub fn new_mock() -> Self {
        let client = Arc::new(Mutex::new(RpcClient::new_mock()));
        Self(client)
    }

    pub async fn clear_cache(&self) {
        let mut rpc_agent = self.0.lock().await;
        rpc_agent.clear_cache();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub async fn create_rpc_client_test(version: &str) -> RpcAgent {
        let rpc = "https://eth.merkle.io";
        let abi_file = File::open(format!(
            "../subgraph-testing/packages/v{version}/abis/ERC20.json"
        ))
        .unwrap();
        let abi = serde_json::from_reader(abi_file).unwrap();
        let mut abis = ABIs::default();
        abis.insert("ERC20".to_string(), abi);

        let client = ethereum::EthereumRPC::new(rpc, abis).await.unwrap();
        let block_ptr = BlockPtr {
            number: 18362011,
            hash: "0xd5f60b37e43ee04d875dc50a3587915863eba289f88a133cfbcbe79733e3bee8".to_string(),
            parent_hash: "0x12bc04af20d07664aae1e09846aa0b1bf344b42f4c1dbb9b2e25c3a4c1dc36f8"
                .to_string(),
        };
        let chain = RPCChain::Ethereum(client);
        let client = RpcClient {
            rpc_client: chain,
            block_ptr,
        };

        RpcAgent(Arc::new(Mutex::new(client)))
    }
}
