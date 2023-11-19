use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::config::Config;
use crate::errors::RPCClientError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
// use tokio::sync::Mutex;
use std::sync::Mutex;
mod ethereum;

#[derive(Clone)]
pub enum RPCChain {
    None,
    Ethereum(ethereum::EthereumRPC),
}

#[async_trait]
impl RPCTrait for RPCChain {
    async fn handle_request(
        &mut self,
        request: CallRequest,
        block_ptr: BlockPtr,
    ) -> Result<CallResponse, RPCClientError> {
        match self {
            RPCChain::Ethereum(client) => client.handle_request(request, block_ptr).await,
            RPCChain::None => Err(RPCClientError::RPCClient(
                "RPCClient is not configured".to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CallRequest {
    EthereumContractCall(UnresolvedContractCall),
}

#[derive(Clone, Debug)]
pub enum CallResponse {
    EthereumContractCall(Option<Vec<ethabi::Token>>),
}

#[async_trait]
pub trait RPCTrait {
    async fn handle_request(
        &mut self,
        call: CallRequest,
        block_ptr: BlockPtr,
    ) -> Result<CallResponse, RPCClientError>;
}

pub struct RpcAgent {
    rpc_client: RPCChain,
    block_ptr: BlockPtr,
}

impl RpcAgent {
    async fn new(
        config: &Config,
        abis: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RPCClientError> {
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

    pub fn handle_request(&mut self, call: CallRequest) -> Result<CallResponse, RPCClientError> {
        #[cfg(test)]
        {
            return async_std::task::block_on(
                self.rpc_client.handle_request(call, self.block_ptr.clone()),
            );
        }
        let handler = tokio::runtime::Handle::current();
        handler.block_on(self.rpc_client.handle_request(call, self.block_ptr.clone()))
    }

    pub fn set_block_ptr(&mut self, block_ptr: BlockPtr) {
        self.block_ptr = block_ptr;
    }

    pub fn new_mock() -> Self {
        Self {
            rpc_client: RPCChain::None,
            block_ptr: BlockPtr::default(),
        }
    }
}

#[derive(Clone)]
pub struct RPCWrapper {
    rpc_agent: Arc<Mutex<RpcAgent>>,
}

impl RPCWrapper {
    pub async fn new(
        config: &Config,
        abis: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RPCClientError> {
        let rpc_client = RpcAgent::new(config, abis).await?;
        Ok(Self {
            rpc_agent: Arc::new(Mutex::new(rpc_client)),
        })
    }

    pub fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCClientError> {
        let mut rpc_agent = self.rpc_agent.lock().unwrap();
        rpc_agent.handle_request(call)
    }

    pub fn set_block_ptr(&self, block_ptr: BlockPtr) {
        let mut rpc_agent = self.rpc_agent.lock().unwrap();
        rpc_agent.set_block_ptr(block_ptr);
    }

    pub fn new_mock() -> Self {
        let agent = Arc::new(Mutex::new(RpcAgent::new_mock()));
        Self { rpc_agent: agent }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::Arc;
    use std::sync::Mutex;

    pub async fn create_rpc_client_test() -> RPCWrapper {
        let rpc = "https://eth.llamarpc.com";
        let abi_file = File::open("./src/tests/abis/aladin.json").unwrap();
        let abi = serde_json::from_reader(abi_file).unwrap();
        let mut abis: HashMap<String, serde_json::Value> = HashMap::new();
        abis.insert("ERC20".to_string(), abi);

        let client = ethereum::EthereumRPC::new(rpc, abis).await.unwrap();
        let block_ptr = BlockPtr {
            number: 18362011,
            hash: "0xd5f60b37e43ee04d875dc50a3587915863eba289f88a133cfbcbe79733e3bee8".to_string(),
            parent_hash: "0x12bc04af20d07664aae1e09846aa0b1bf344b42f4c1dbb9b2e25c3a4c1dc36f8"
                .to_string(),
        };
        let chain = RPCChain::Ethereum(client);
        let agent = RpcAgent {
            rpc_client: chain,
            block_ptr,
        };

        RPCWrapper {
            rpc_agent: Arc::new(Mutex::new(agent)),
        }
    }
}
