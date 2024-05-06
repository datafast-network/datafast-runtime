use super::types::CallRequest;
use super::types::CallRequestContext;
use super::types::CallResponse;
use super::RPCTrait;
use crate::chain::ethereum::ethereum_call::EthereumContractCall;
use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::ABIs;
use crate::common::BlockPtr;
use crate::error;
use crate::errors::RPCError;
use crate::info;
use async_trait::async_trait;
use std::collections::HashMap;
use std::str::FromStr;
use web3::transports::WebSocket;
use web3::types::Block;
use web3::types::BlockId;
use web3::types::BlockNumber;
use web3::types::H256;
use web3::Web3;

const ETH_CALL_GAS: u32 = 50_000_000;

impl<T> From<Block<T>> for BlockPtr {
    fn from(b: Block<T>) -> Self {
        Self {
            number: b.number.unwrap().as_u64(),
            hash: format!("{:?}", b.hash.unwrap()),
            parent_hash: format!("{:?}", b.parent_hash),
        }
    }
}

#[derive(Default)]
struct CacheRPC(HashMap<CallRequest, CallResponse>);

pub struct EthereumRPC {
    client: Web3<WebSocket>,
    supports_eip_1898: bool,
    abis: ABIs,
    cache: CacheRPC,
}

impl EthereumRPC {
    pub async fn new(url: &str, abis: ABIs) -> Result<Self, RPCError> {
        let client = Web3::new(WebSocket::new(url).await.unwrap());
        let supports_eip_1898 = client
            .web3()
            .client_version()
            .await
            .map(|s| s.contains("TestRPC"))
            .unwrap_or(false);
        info!(EthereumRPC, "client check"; supports_eip_1898 => supports_eip_1898);
        Ok(EthereumRPC {
            client,
            supports_eip_1898,
            abis,
            cache: CacheRPC::default(),
        })
    }

    fn parse_contract_call_request(
        &self,
        call: UnresolvedContractCall,
    ) -> Result<EthereumContractCall, RPCError> {
        let contract_name = call.contract_name;
        let function_name = call.function_name;
        let contract_address = call.contract_address;

        //get contract abi
        let abi = self.abis.get_contract(&contract_name).ok_or_else(|| {
            error!(
                RPCError,
                "get abi failed";
                contract_name => contract_name,
                function_name => function_name,
                contract_address => contract_address
            );
            RPCError::BadABI
        })?;

        let function_call = match call.function_signature {
            // Behavior for apiVersion < 0.0.4: look up function by name; for overloaded
            // functions this always picks the same overloaded variant, which is incorrect
            // and may lead to encoding/decoding errors
            None => abi.function(&function_name).map_err(|e| {
                error!(
                    handle_call_request,
                    "Contract function not found";
                    contract_name => contract_name,
                    function_name => function_name,
                    contract_address => contract_address,
                    e => format!("{:?}", e)
                );
                RPCError::FunctionNotFound
            })?,

            // Behavior for apiVersion >= 0.0.04: look up function by signature of
            // the form `functionName(uint256,string) returns (bytes32,string)`; this
            // correctly picks the correct variant of an overloaded function
            Some(fn_signature) => abi
                .functions_by_name(&function_name)
                .map_err(|e| {
                    error!(
                        handle_call_request,
                        "Contract function not found";
                        contract_name => contract_name,
                        function_name => function_name,
                        contract_address => contract_address,
                        function_signature => fn_signature,
                        error => format!("{:?}", e)
                    );
                    RPCError::FunctionNotFound
                })?
                .iter()
                .find(|f| f.signature() == fn_signature)
                .ok_or_else(|| {
                    error!(
                        handle_call_request,
                        "Contract function not found";
                        contract_name => contract_name,
                        function_name => function_name,
                        contract_address => contract_address,
                        function_signature => fn_signature
                    );
                    RPCError::SignatureNotFound
                })?,
        };

        let result = EthereumContractCall {
            address: contract_address,
            function: function_call.clone(),
            args: call.function_args,
        };

        // Emit custom error for type mismatches.
        for (token, kind) in result
            .args
            .iter()
            .zip(result.function.inputs.iter().map(|p| &p.kind))
        {
            if !token.type_check(kind) {
                return Err(RPCError::InvalidArguments);
            }
        }

        Ok(result)
    }

    async fn handle_contract_call(
        &self,
        data: UnresolvedContractCall,
        block_ptr: BlockPtr,
    ) -> Result<CallResponse, RPCError> {
        assert!(block_ptr.number > 0, "bad block");
        let request_data = self.parse_contract_call_request(data)?;
        // Encode the call parameters according to the ABI
        let call_data = request_data
            .function
            .encode_input(&request_data.args)
            .map(web3::types::Bytes::from)
            .map_err(|e| {
                error!(
                    ethereum_call,
                    "Contract function call failed";
                    error => format!("{:?}", e),
                    contract_address => format!("{:?}", request_data.address),
                    function_name => format!("{:?}", request_data.function.name),
                    block_number => block_ptr.number,
                    block_hash => block_ptr.hash
                );
                RPCError::Revert(format!("{:?}", e))
            })?;

        let block_id = if !self.supports_eip_1898 {
            BlockId::Number(block_ptr.number.into())
        } else {
            BlockId::Hash(H256::from_str(&block_ptr.hash).unwrap())
        };

        let request = web3::types::CallRequest {
            to: Some(request_data.address),
            gas: Some(web3::types::U256::from(ETH_CALL_GAS)),
            data: Some(call_data),
            from: None,
            gas_price: None,
            value: None,
            access_list: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            transaction_type: None,
        };

        let result = self
            .client
            .eth()
            .call(request.clone(), Some(block_id))
            .await
            .map_err(|e| {
                error!(
                    ethereum_call,
                    "calling contract function failed";
                    error => format!("{:?}", e),
                    contract_address => format!("{:?}", request_data.address),
                    function_name => format!("{:?}", request_data.function.name),
                    block_number => block_ptr.number,
                    block_hash => block_ptr.hash
                );
                RPCError::ContractCallFail
            })?;

        let result = request_data
            .function
            .decode_output(&result.0)
            .map(CallResponse::EthereumContractCall)
            .map_err(|e| {
                error!(
                    ethereum_call,
                    "Decoding contract function call failed";
                    error => format!("{:?}", e),
                    contract_address => format!("{:?}", request_data.address),
                    function_name => format!("{:?}", request_data.function.name),
                    block_number => block_ptr.number,
                    block_hash => block_ptr.hash
                );
                RPCError::Revert(format!("{:?}", e))
            })?;

        Ok(result)
    }
}

#[async_trait]
impl RPCTrait for EthereumRPC {
    async fn handle_request(&mut self, call: CallRequestContext) -> Result<CallResponse, RPCError> {
        match call.call_request {
            CallRequest::EthereumContractCall(data) => {
                self.handle_contract_call(data.clone(), call.block_ptr.clone())
                    .await
            }
        }
    }

    async fn get_latest_block(&mut self) -> Result<BlockPtr, RPCError> {
        self.client
            .eth()
            .block(BlockId::Number(BlockNumber::Latest))
            .await
            .map_err(|e| {
                error!(EthereumRPC, "get latest block failed"; error => e);
                RPCError::GetLatestBlockFail
            })?
            .map(|b| Ok(BlockPtr::from(b)))
            .unwrap()
    }

    fn cache_get(&self, call: &CallRequest) -> Option<CallResponse> {
        self.cache.0.get(call).cloned()
    }

    fn cache_set(&mut self, call: &CallRequest, result: &CallResponse) {
        self.cache.0.insert(call.clone(), result.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
    use crate::common::ABIs;
    use crate::common::BlockPtr;
    use df_logger::log;
    use df_logger::loggers::init_logger;
    use ethabi::Address;
    use ethabi::Token;
    use std::fs;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_rpc_call_symbol() {
        init_logger();
        let data = UnresolvedContractCall {
            contract_name: "ERC20".to_string(),
            contract_address: Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7")
                .unwrap(),
            function_name: "symbol".to_string(),
            function_signature: None,
            function_args: vec![],
        };
        let abi =
            fs::read_to_string("../subgraph-testing/packages/uniswap-v3/build/NonfungiblePositionManager/abis/ERC20.json").unwrap();
        let mut abis = ABIs::default();
        abis.insert("ERC20".to_string(), serde_json::from_str(&abi).unwrap());
        let rpc = EthereumRPC::new("wss://eth.merkle.io", abis).await.unwrap();
        let block_ptr = BlockPtr {
            number: 18_500_000,
            hash: "0x80ce6bb0e244fbdf66cf0a1108273fe1ca58788efb7fb8d3a0d783d2b06d433d".to_string(),
            parent_hash: "0x38e2aa07d0d1e3c9e5d0dd74d87dfd6a2f3981c6caa44c098eb0b55f3e04d99f"
                .to_string(),
        };
        let result = rpc.handle_contract_call(data, block_ptr).await.unwrap();

        assert_eq!(
            result,
            CallResponse::EthereumContractCall(vec![Token::String("USDT".to_string())])
        );
    }

    #[tokio::test]
    async fn test_get_latest_block() {
        init_logger();
        let mut rpc = EthereumRPC::new("wss://eth.merkle.io", ABIs::default())
            .await
            .unwrap();
        log::info!("{:?}", rpc.get_latest_block().await.unwrap());
    }
}
