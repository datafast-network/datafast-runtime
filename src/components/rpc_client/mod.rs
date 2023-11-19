use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::config::Config;
use crate::errors::RPCClientError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[derive(Clone)]
pub struct RpcAgent {
    rpc_client: Arc<Mutex<RPCChain>>,
    block_ptr: BlockPtr,
}

impl RpcAgent {
    pub async fn new(
        config: &Config,
        abis: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RPCClientError> {
        let client = match config.chain {
            Chain::Ethereum => {
                let client = ethereum::EthereumRPC::new(&config.rpc_endpoint, abis).await?;
                RPCChain::Ethereum(client)
            }
        };
        let rpc_client = Arc::new(Mutex::new(client));
        Ok(Self {
            rpc_client,
            block_ptr: BlockPtr::default(),
        })
    }

    pub fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCClientError> {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            let mut rpc_client = self.rpc_client.lock().await;
            rpc_client
                .handle_request(call, self.block_ptr.clone())
                .await
        })
    }

    pub fn set_block_ptr(&mut self, block_ptr: BlockPtr) {
        self.block_ptr = block_ptr;
    }

    pub fn new_mock() -> Self {
        Self {
            rpc_client: Arc::new(Mutex::new(RPCChain::None)),
            block_ptr: BlockPtr::default(),
        }
    }
}
