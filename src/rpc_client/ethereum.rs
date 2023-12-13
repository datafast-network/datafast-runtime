use super::types::CallRequest;
use super::types::CallRequestContext;
use super::types::CallResponse;
use super::RPCTrait;
use crate::chain::ethereum::ethereum_call::EthereumContractCall;
use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::error;
use crate::errors::RPCClientError;
use crate::info;
use async_trait::async_trait;
use std::collections::HashMap;
use std::str::FromStr;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use web3::transports::Http;
use web3::types::BlockId;
use web3::types::H256;
use web3::Web3;

const ETH_CALL_GAS: u32 = 50_000_000;

#[derive(Clone)]
pub struct EthereumRPC {
    client: Web3<Http>,
    supports_eip_1898: bool,
    abis: HashMap<String, ethabi::Contract>,
}

impl EthereumRPC {
    pub async fn new(
        url: &str,
        abis: HashMap<String, serde_json::Value>,
    ) -> Result<Self, RPCClientError> {
        let client = Web3::new(Http::new(url).unwrap());
        let abis = abis
            .iter()
            .map(|(contract_name, abi)| {
                (
                    contract_name.clone(),
                    serde_json::from_value(abi.clone()).expect("invalid abi"),
                )
            })
            .collect();
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
        let abi = self
            .abis
            .iter()
            .find(|(name, _)| **name == contract_name)
            .ok_or_else(|| {
                error!(
                    RPCClientError,
                    "get abi failed";
                    contract_name => contract_name,
                    function_name => function_name,
                    contract_address => contract_address
                );
                RPCClientError::BadABI
            })?
            .1;

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
                    e => e
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
        let request_data = self.parse_contract_call_request(data)?;
        // Encode the call parameters according to the ABI
        let call_data = match request_data.function.encode_input(&request_data.args) {
            Ok(data) => web3::types::Bytes(data),
            Err(e) => {
                error!(
                    ethereum_call,
                    "Contract function call failed";
                    error => format!("{:?}", e),
                    contract_address => format!("{:?}", request_data.address),
                    function_name => format!("{:?}", request_data.function.name),
                    block_number => block_ptr.number,
                    block_hash => block_ptr.hash
                );
                return Err(RPCClientError::DataEncodingFail);
            }
        };

        let block_id = if !self.supports_eip_1898 {
            BlockId::Number(block_ptr.number.into())
        } else {
            BlockId::Hash(H256::from_str(&block_ptr.hash).unwrap())
        };

        let req = web3::types::CallRequest {
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

        let result = Retry::spawn(FixedInterval::from_millis(100).take(5), || {
            self.client.eth().call(req.clone(), Some(block_id))
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

        let data_result = request_data
            .function
            .decode_output(&result.0)
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
                RPCClientError::DatDecodingFail
            })?;
        let response = CallResponse::EthereumContractCall(Some(data_result));
        Ok(response)
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
