use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use std::collections::HashMap;

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

pub type RPCCache = HashMap<CallRequestContext, CallResponse>;
