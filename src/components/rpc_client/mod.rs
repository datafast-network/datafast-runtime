use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::common::Chain;
use crate::errors::RPCClientError;
use async_trait::async_trait;
use std::collections::HashMap;

mod ethereum;

#[derive(Clone)]
pub enum RPCChain {
    None,
    Ethereum(ethereum::EthereumRPC),
}

impl RPCChain {
    pub async fn new(
        url: &str,
        chain: Chain,
        abis: HashMap<String, ethabi::Contract>,
    ) -> Result<Self, RPCClientError> {
        let client = match chain {
            Chain::Ethereum => {
                let client = ethereum::EthereumRPC::new(url, abis).await?;
                RPCChain::Ethereum(client)
            }
        };
        Ok(client)
    }

    pub fn handle_request(&self, request: CallRequest) -> Result<CallResponse, RPCClientError> {
        match self {
            RPCChain::Ethereum(client) => {
                let handle = tokio::runtime::Handle::current();
                match request {
                    CallRequest::EthereumContractCall { .. } => {
                        handle.block_on(client.handle_request(request))
                    }
                }
            }
            RPCChain::None => Err(RPCClientError::RPCClient(
                "RPCClient is not configured".to_string(),
            )),
        }
    }

    pub fn set_block_ptr(&mut self, block_ptr: BlockPtr) {
        match self {
            RPCChain::Ethereum(client) => client.set_block_ptr(block_ptr),
            RPCChain::None => {}
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
    async fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCClientError>;

    fn set_block_ptr(&mut self, block_ptr: BlockPtr);
}
