use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::config::Config;
use crate::errors::RPCClientError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use web3::futures::executor;
mod ethereum;

type RPCCache = HashMap<String, CallResponse>;
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
        let cache = self.get_cache_instance();
        let cache_key = self.get_cache_key(&request, &block_ptr);
        if cache.contains_key(&cache_key) {
            return Ok(cache.get(&cache_key).unwrap().clone());
        }
        match self {
            RPCChain::Ethereum(client) => {
                let response = client.handle_request(request, block_ptr).await?;
                client.set_cache(response.clone(), cache_key);
                Ok(response)
            }
            RPCChain::None => Err(RPCClientError::RPCClient(
                "RPCClient is not configured".to_string(),
            )),
        }
    }

    fn get_cache_instance(&self) -> &RPCCache {
        match self {
            RPCChain::Ethereum(client) => client.get_cache_instance(),
            RPCChain::None => {
                panic!("RPCClient is not configured");
            }
        }
    }

    fn set_cache(&mut self, call_response: CallResponse, cache_key: String) {
        match self {
            RPCChain::Ethereum(client) => client.set_cache(call_response, cache_key),
            RPCChain::None => {
                panic!("RPCClient is not configured");
            }
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

    fn get_cache_instance(&self) -> &RPCCache;

    fn get_cache_key(&self, request: &CallRequest, block_ptr: &BlockPtr) -> String {
        let mut keys = vec![block_ptr.number.to_string(), block_ptr.hash.clone()];
        match request {
            CallRequest::EthereumContractCall(call) => {
                //Key = block_number + block_hash + contract_name + contract_address + function_name + function_args
                keys.push(call.contract_name.clone());
                keys.push(format!("{:?}", call.contract_address));
                keys.push(call.function_name.clone());
                let args = call
                    .function_args
                    .iter()
                    .map(|token| token.to_string())
                    .collect::<Vec<String>>()
                    .join("_");
                keys.push(args);
            }
        }
        hex::encode(keys.join(""))
    }

    fn set_cache(&mut self, call_response: CallResponse, cache_key: String);
}

pub struct RpcClient {
    rpc_client: RPCChain,
    block_ptr: BlockPtr,
}

impl RpcClient {
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

    pub async fn handle_request(
        &mut self,
        call: CallRequest,
    ) -> Result<CallResponse, RPCClientError> {
        self.rpc_client
            .handle_request(call, self.block_ptr.clone())
            .await
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
pub struct RpcAgent {
    client: Arc<Mutex<RpcClient>>,
}

impl RpcAgent {
    pub async fn new(
        config: &Config,
        abis: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RPCClientError> {
        let rpc_client = RpcClient::new(config, abis).await?;
        Ok(Self {
            client: Arc::new(Mutex::new(rpc_client)),
        })
    }

    pub fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCClientError> {
        executor::block_on(async {
            let mut rpc_agent = self.client.lock().await;
            rpc_agent.handle_request(call).await
        })
    }

    pub async fn set_block_ptr(&self, block_ptr: BlockPtr) {
        let mut rpc_agent = self.client.lock().await;
        rpc_agent.set_block_ptr(block_ptr);
    }

    pub fn new_mock() -> Self {
        let client = Arc::new(Mutex::new(RpcClient::new_mock()));
        Self { client }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub async fn create_rpc_client_test(version: &str) -> RpcAgent {
        let rpc = "https://eth.llamarpc.com";
        let abi_file = File::open(format!(
            "../subgraph-testing/packages/v{version}/abis/ERC20.json"
        ))
        .unwrap();
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
        let client = RpcClient {
            rpc_client: chain,
            block_ptr,
        };

        RpcAgent {
            client: Arc::new(Mutex::new(client)),
        }
    }
}