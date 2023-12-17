use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CallRequestContext {
    pub block_ptr: BlockPtr,
    pub call_request: CallRequest,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CallRequest {
    EthereumContractCall(UnresolvedContractCall),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CallResponse {
    EthereumContractCall(Vec<ethabi::Token>),
}

impl CallRequest {
    pub fn is_cachable(&self) -> bool {
        match self {
            CallRequest::EthereumContractCall(call) => {
                todo!()
            }
        }
    }
}
