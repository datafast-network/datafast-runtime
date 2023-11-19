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
    cache: HashMap<String, CallResponse>,
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
        log::info!("get client version");
        let supports_eip_1898 = client
            .web3()
            .client_version()
            .await
            .map(|s| s.contains("TestRPC"))
            .unwrap_or(false);
        log::info!("supports_eip_1898: {:?}", supports_eip_1898);
        Ok(EthereumRPC {
            client,
            supports_eip_1898,
            abis,
            cache: HashMap::new(),
        })
    }

    fn handle_call_request(
        &self,
        call: UnresolvedContractCall,
        block_ptr: BlockPtr,
    ) -> Result<(String, EthereumContractCall), RPCClientError> {
        let contract_name = call.contract_name;
        let function_name = call.function_name;
        let contract_address = call.contract_address;
        let args = call
            .function_args
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>();

        let key = hex::encode(format!(
            "{}{}{}{}{}{}",
            contract_name,
            function_name,
            contract_address,
            args.join(","),
            block_ptr.clone().number,
            block_ptr.clone().hash
        ));

        //get contract abi
        let abi = self
            .abis
            .iter()
            .find(|(name, _)| **name == contract_name)
            .ok_or_else(|| {
                RPCClientError::RPCClient(format!("Contract \"{}\" not found", contract_name))
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

        Ok((
            key,
            EthereumContractCall {
                address: contract_address,
                function: function_call.clone(),
                args: call.function_args,
            },
        ))
    }

    async fn contract_call(
        &mut self,
        request_data: EthereumContractCall,
        block_ptr: BlockPtr,
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
        // let key =
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
        let response = CallResponse::EthereumContractCall(Some(data_result));
        self.cache
            .insert(request_data.function.name.clone(), response.clone());
        Ok(response)
    }
}

#[async_trait]
impl RPCTrait for EthereumRPC {
    async fn handle_request(
        &mut self,
        call: CallRequest,
        block_ptr: BlockPtr,
    ) -> Result<CallResponse, RPCClientError> {
        match call {
            CallRequest::EthereumContractCall(data) => {
                let (key, request_data) = self.handle_call_request(data, block_ptr.clone())?;

                if let Some(response) = self.cache.get(&key) {
                    return Ok(response.clone());
                }

                self.contract_call(request_data, block_ptr).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    async fn test_contract_call_rpc_client() {
        env_logger::try_init().unwrap_or_default();
        let rpc = "https://eth.llamarpc.com";
        let abi_file = File::open(
            "/Users/vutran/Works/hardbed/subgraph-testing/packages/v0_0_5/abis/ERC20.json",
        )
        .unwrap();
        let abi = serde_json::from_reader(abi_file).unwrap();
        let mut abis: HashMap<String, serde_json::Value> = HashMap::new();
        abis.insert("ERC20".to_string(), abi);
        let mut rpc_client = EthereumRPC::new(rpc, abis).await.unwrap();
        let block_ptr = BlockPtr {
            number: 18362011,
            hash: "0xd5f60b37e43ee04d875dc50a3587915863eba289f88a133cfbcbe79733e3bee8".to_string(),
            parent_hash: "0x12bc04af20d07664aae1e09846aa0b1bf344b42f4c1dbb9b2e25c3a4c1dc36f8"
                .to_string(),
        };
        // rpc_client
        let call_request = CallRequest::EthereumContractCall(UnresolvedContractCall {
            contract_name: "ERC20".to_string(),
            contract_address: web3::types::Address::from_str(
                "0x95a41fb80ca70306e9ecf4e51cea31bd18379c18",
            )
            .unwrap(),
            function_name: "symbol".to_string(),
            function_signature: None,
            function_args: vec![],
        });

        let start = tokio::time::Instant::now();

        let result = rpc_client
            .handle_request(call_request, block_ptr)
            .await
            .unwrap();
        match result {
            CallResponse::EthereumContractCall(Some(tokens)) => {
                assert_eq!(tokens.len(), 1);
                assert_eq!(tokens[0].to_string(), "ADN");
            }
            _ => panic!("should not happen"),
        }
        log::info!("time: {:?}", start.elapsed());
    }

    #[rstest::rstest]
    fn test_call_rstest() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(test_contract_call_rpc_client());
    }
}
