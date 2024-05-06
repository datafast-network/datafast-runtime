use std::fmt::Display;

use df_types::chain::ethereum::ethereum_call::UnresolvedContractCall;
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
            CallRequest::EthereumContractCall(UnresolvedContractCall {
                contract_name,
                function_name,
                ..
            }) => match (contract_name.as_str(), function_name.as_str()) {
                (
                    "ERC20" | "ERC20NameBytes" | "ERC20SymbolBytes",
                    "symbol" | "decimals" | "name",
                ) => true,
                _ => false,
            },
        }
    }
}

impl Display for CallRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallRequest::EthereumContractCall(call) => {
                write!(
                    f,
                    "contract_name={}, function={}, address={:?}",
                    call.contract_name, call.function_name, call.contract_address
                )
            }
        }
    }
}
