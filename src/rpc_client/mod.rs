mod ethereum;
mod metrics;
mod types;

use self::metrics::RpcMetrics;
use crate::common::ABIs;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::config::Config;
use crate::errors::RPCError;
use async_trait::async_trait;
use prometheus::Registry;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
pub use types::*;

#[async_trait]
pub trait RPCTrait {
    async fn handle_request(&mut self, call: CallRequestContext) -> Result<CallResponse, RPCError>;
    async fn get_latest_block(&mut self) -> Result<BlockPtr, RPCError>;
    fn cache_get(&self, call: &CallRequest) -> Option<CallResponse>;
    fn cache_set(&mut self, call: &CallRequest, result: &CallResponse);
}

pub enum RPCChain {
    #[allow(dead_code)]
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

    fn cache_set(&mut self, call: &CallRequest, result: &CallResponse) {
        match self {
            RPCChain::Ethereum(client) => client.cache_set(call, result),
            RPCChain::None => (),
        }
    }
}

pub struct RpcClient {
    rpc_client: RPCChain,
    block_ptr: BlockPtr,
    cache_by_block: HashMap<CallRequestContext, CallResponse>,
    metrics: RpcMetrics,
}

impl RpcClient {
    async fn new(config: &Config, abis: ABIs, registry: &Registry) -> Result<Self, RPCError> {
        let rpc_client = match config.chain {
            Chain::Ethereum => {
                let client = ethereum::EthereumRPC::new(&config.rpc_endpoint, abis).await?;
                RPCChain::Ethereum(client)
            }
        };
        Ok(Self {
            rpc_client,
            block_ptr: BlockPtr::default(),
            cache_by_block: HashMap::new(),
            metrics: RpcMetrics::new(registry),
        })
    }

    fn new_mock(registry: &Registry) -> Self {
        Self {
            rpc_client: RPCChain::None,
            block_ptr: BlockPtr::default(),
            cache_by_block: HashMap::new(),
            metrics: RpcMetrics::new(registry),
        }
    }

    pub async fn handle_request(&mut self, call: CallRequest) -> Result<CallResponse, RPCError> {
        if let Some(result) = self.rpc_client.cache_get(&call) {
            self.metrics.chain_level_cache_hit.inc();
            return Ok(result);
        }

        let is_chain_level_cachable = call.is_cachable();

        let call_context = CallRequestContext {
            block_ptr: self.block_ptr.clone(),
            call_request: call.clone(),
        };

        if let Some(result) = self.cache_by_block.get(&call_context) {
            self.metrics.block_level_cache_hit.inc();
            return Ok(result.clone());
        }

        let timer = self.metrics.rpc_request_duration.start_timer();
        let result = self.rpc_client.handle_request(call_context.clone()).await?;
        self.cache_by_block.insert(call_context, result.clone());
        timer.stop_and_record();

        if is_chain_level_cachable {
            self.metrics.chain_level_cache_miss.inc();
            self.rpc_client.cache_set(&call, &result);
        } else {
            self.metrics.block_level_cache_miss.inc();
        }

        Ok(result)
    }

    pub fn set_block_ptr(&mut self, block_ptr: &BlockPtr) {
        self.block_ptr = block_ptr.clone();
    }

    pub fn clear_block_level_cache(&mut self) {
        self.cache_by_block = HashMap::new()
    }
}

#[derive(Clone)]
pub struct RpcAgent(Rc<RefCell<RpcClient>>);

unsafe impl Send for RpcAgent {}

impl RpcAgent {
    pub async fn new(config: &Config, abis: ABIs, registry: &Registry) -> Result<Self, RPCError> {
        let rpc_client = RpcClient::new(config, abis, registry).await?;
        Ok(Self(Rc::new(RefCell::new(rpc_client))))
    }

    pub fn new_mock(registry: &Registry) -> Self {
        let rpc_client = RpcClient::new_mock(&registry);
        Self(Rc::new(RefCell::new(rpc_client)))
    }

    pub fn handle_request(&mut self, call: CallRequest) -> Result<CallResponse, RPCError> {
        let mut rpc = self.0.borrow_mut();
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(rpc.handle_request(call))
        })
    }

    pub fn set_block_ptr(&mut self, block_ptr: &BlockPtr) {
        let mut rpc = self.0.borrow_mut();
        rpc.set_block_ptr(block_ptr);
    }

    pub fn clear_block_level_cache(&mut self) {
        let mut rpc = self.0.borrow_mut();
        rpc.clear_block_level_cache();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;

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
            cache_by_block: HashMap::new(),
            metrics: RpcMetrics::new(&Registry::new()),
        };

        RpcAgent(Rc::new(RefCell::new(client)))
    }
}
