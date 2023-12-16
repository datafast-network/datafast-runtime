use super::types::CallRequest;
use super::types::CallRequestContext;
use super::types::CallResponse;
use super::RPCTrait;
use crate::chain::ethereum::ethereum_call::EthereumContractCall;
use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::ABIs;
use crate::common::BlockPtr;
use crate::error;
use crate::errors::RPCClientError;
use crate::info;
use async_trait::async_trait;
use std::str::FromStr;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use web3::transports::WebSocket;
use web3::types::BlockId;
use web3::types::H256;
use web3::Web3;

const ETH_CALL_GAS: u32 = 50_000_000;

#[derive(Clone)]
pub struct EthereumRPC {
    client: Web3<WebSocket>,
    supports_eip_1898: bool,
    abis: ABIs,
}

impl EthereumRPC {
    pub async fn new(url: &str, abis: ABIs) -> Result<Self, RPCClientError> {
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
        })
    }

    fn parse_contract_call_request(
        &self,
        call: UnresolvedContractCall,
    ) -> Result<EthereumContractCall, RPCClientError> {
        let contract_name = call.contract_name;
        let function_name = call.function_name;
        let contract_address = call.contract_address;

        //get contract abi
        let abi = self.abis.get_contract(&contract_name).ok_or_else(|| {
            error!(
                RPCClientError,
                "get abi failed";
                contract_name => contract_name,
                function_name => function_name,
                contract_address => contract_address
            );
            RPCClientError::BadABI
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
                RPCClientError::FunctionNotFound
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
                    RPCClientError::FunctionNotFound
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
                    RPCClientError::SignatureNotFound
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
                return Err(RPCClientError::InvalidArguments);
            }
        }

        Ok(result)
    }

    async fn handle_contract_call(
        &self,
        data: UnresolvedContractCall,
        block_ptr: BlockPtr,
    ) -> Result<CallResponse, RPCClientError> {
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
                RPCClientError::Revert(format!("{:?}", e))
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

        let result = Retry::spawn(FixedInterval::from_millis(5).take(5), || {
            self.client.eth().call(request.clone(), Some(block_id))
        })
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
            RPCClientError::ContractCallFail
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
                RPCClientError::Revert(format!("{:?}", e))
            })?;

        Ok(result)
    }
}

#[async_trait]
impl RPCTrait for EthereumRPC {
    async fn handle_request(
        &mut self,
        call: CallRequestContext,
    ) -> Result<CallResponse, RPCClientError> {
        match call.call_request {
            CallRequest::EthereumContractCall(data) => {
                self.handle_contract_call(data, call.block_ptr).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
    use crate::common::ABIs;
    use ethabi::Address;
    use std::fs;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_rpc_call_symbol() {
        env_logger::try_init().unwrap_or_default();
        let data = UnresolvedContractCall {
            contract_name: "ERC20".to_string(),
            contract_address: Address::from_str("0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2")
                .unwrap(),
            function_name: "symbol".to_string(),
            function_signature: None,
            function_args: vec![],
        };
        let abi =
            fs::read_to_string("./subgraph/NonfungiblePositionManager/abis/ERC20.json").unwrap();
        let mut abis = ABIs::default();
        abis.insert("ERC20".to_string(), serde_json::from_str(&abi).unwrap());
        let rpc = super::EthereumRPC::new("https://eth.llamarpc.com", abis)
            .await
            .unwrap();
        let block_ptr = crate::common::BlockPtr {
            number: 12369879,
            hash: "0x7d81e60e5a2296dc38f36e343a7f3e416b1fc2f766568b2d81a63159752b8885".to_string(),
            parent_hash: "0x6c768e2debe6d3cb09e078387c20ea90b41e3899ecd0f65e523be9f9bb0033b7"
                .to_string(),
        };
        let result = rpc.handle_contract_call(data, block_ptr).await.unwrap();
        log::info!("result: {:?}", result);
    }
}
