use crate::chain::ethereum::ethereum_call::EthereumContractCall;
use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::common::BlockPtr;
use crate::components::rpc_client::CallRequest;
use crate::components::rpc_client::CallResponse;
use crate::components::rpc_client::RPCTrait;
use crate::error;
use crate::errors::RPCClientError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::str::FromStr;
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
    block_ptr: Option<BlockPtr>,
    //TODO: add cache result into memory or db
}

impl EthereumRPC {
    pub async fn new(
        url: &str,
        abis: HashMap<String, ethabi::Contract>,
    ) -> Result<Self, RPCClientError> {
        let client = Web3::new(Http::new(url).unwrap());

        let supports_eip_1898 = client
            .web3()
            .client_version()
            .await
            .map(|s| s.contains("TestRPC"))
            .unwrap_or(false);

        Ok(EthereumRPC {
            client,
            supports_eip_1898,
            abis,
            block_ptr: None,
        })
    }

    fn handle_call_request(
        &self,
        call: UnresolvedContractCall,
    ) -> Result<EthereumContractCall, RPCClientError> {
        if self.block_ptr.is_none() {
            return Err(RPCClientError::RPCClient(
                "Block pointer is not set".to_string(),
            ));
        }

        //get contract abi
        let abi = self
            .abis
            .iter()
            .find(|(name, contract)| **name == call.contract_name)
            .ok_or_else(|| {
                RPCClientError::RPCClient(format!("Contract \"{}\" not found", call.contract_name))
            })?
            .1;

        let function_name = call.function_name;
        let contract_name = call.contract_name;
        let contract_address = call.contract_address;

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
                    error => format!("{:?}", e)
                );
                RPCClientError::RPCClient(e.to_string())
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
                    RPCClientError::RPCClient(e.to_string())
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
                    RPCClientError::RPCClient(format!(
                        "Contract function not found: {}",
                        fn_signature
                    ))
                })?,
        };

        Ok(EthereumContractCall {
            address: contract_address,
            function: function_call.clone(),
            args: call.function_args,
        })
    }

    async fn contract_call(
        &self,
        request_data: EthereumContractCall,
    ) -> Result<CallResponse, RPCClientError> {
        // Emit custom error for type mismatches.
        for (token, kind) in request_data
            .args
            .iter()
            .zip(request_data.function.inputs.iter().map(|p| &p.kind))
        {
            if !token.type_check(kind) {
                return Err(RPCClientError::RPCClient(format!(
                    "Invalid argument {:?} for function {:?}",
                    token, request_data.function
                )));
            }
        }
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
                return Err(RPCClientError::RPCClient(e.to_string()));
            }
        };
        let block_ptr = self.block_ptr.clone().unwrap();

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
        let result = self
            .client
            .eth()
            .call(req, Some(block_id))
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
                RPCClientError::RPCClient(e.to_string())
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
                RPCClientError::RPCClient(e.to_string())
            })?;

        Ok(CallResponse::EthereumContractCall(Some(data_result)))
    }
}

#[async_trait]
impl RPCTrait for EthereumRPC {
    async fn handle_request(&self, call: CallRequest) -> Result<CallResponse, RPCClientError> {
        match call {
            CallRequest::EthereumContractCall(data) => {
                let request_data = self.handle_call_request(data)?;
                self.contract_call(request_data).await
            }
        }
    }

    fn set_block_ptr(&mut self, block_ptr: BlockPtr) {
        self.block_ptr = Some(block_ptr)
    }
}
