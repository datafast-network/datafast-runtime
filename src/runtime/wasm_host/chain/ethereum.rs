use crate::chain::ethereum::asc::EthereumValueKind;
use crate::chain::ethereum::ethereum_call::AscUnresolvedContractCall;
use crate::chain::ethereum::ethereum_call::AscUnresolvedContractCallV4;
use crate::chain::ethereum::ethereum_call::UnresolvedContractCall;
use crate::errors::AscError;
use crate::rpc_client::CallRequest;
use crate::rpc_client::CallResponse;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::r#enum::AscEnum;
use crate::runtime::asc::native_types::r#enum::AscEnumArray;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::asc::native_types::Uint8Array;
use crate::runtime::wasm_host::Env;
use ethabi::decode;
use ethabi::param_type::Reader;
use semver::Version;
use tiny_keccak::Hasher;
use wasmer::FunctionEnvMut;

pub fn ethereum_encode(
    mut fenv: FunctionEnvMut<Env>,
    token: AscPtr<AscEnum<EthereumValueKind>>,
) -> Result<AscPtr<Uint8Array>, AscError> {
    let token_ptr: ethabi::Token = asc_get(&fenv, token, 0)?;

    let bytes = ethabi::encode(&[token_ptr]);
    let asc_bytes = asc_new(&mut fenv, bytes.as_slice())?;
    Ok(asc_bytes)
}

pub fn ethereum_decode(
    mut fenv: FunctionEnvMut<Env>,
    types_ptr: AscPtr<AscString>,
    data_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscEnum<EthereumValueKind>>, AscError> {
    let types: String = asc_get(&fenv, types_ptr, 0)?;
    let data: Vec<u8> = asc_get(&fenv, data_ptr, 0)?;
    let param_types = Reader::read(&types)
        .map_err(|_| AscError::Plain("ethereum decode types error".to_string()))?;

    let data = decode(&[param_types], &data)
        // The `.pop().unwrap()` here is ok because we're always only passing one
        // `param_types` to `decode`, so the returned `Vec` has always size of one.
        // We can't do `tokens[0]` because the value can't be moved out of the `Vec`.
        .map(|mut tokens| tokens.pop().unwrap())
        .map_err(|_| AscError::Plain("ethereum decode token error".to_string()))?;
    let asc_data = asc_new(&mut fenv, &data)?;
    Ok(asc_data)
}

pub fn crypto_keccak_256(
    mut fenv: FunctionEnvMut<Env>,
    input_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<Uint8Array>, AscError> {
    let input: Vec<u8> = asc_get(&fenv, input_ptr, 0)?;
    let data = &input[..];
    let mut hash = tiny_keccak::Keccak::v256();
    let mut output = [0u8; 32];
    hash.update(data);
    hash.finalize(&mut output);
    let hash_256 = web3::types::H256::from_slice(&output);
    let asc_data = asc_new(&mut fenv, &hash_256)?;
    Ok(asc_data)
}

pub fn ethereum_call(
    mut fenv: FunctionEnvMut<Env>,
    wasm_ptr: i32,
) -> Result<AscEnumArray<EthereumValueKind>, AscError> {
    let asc_ptr = wasm_ptr as u32;
    let call: UnresolvedContractCall = if fenv.data().api_version >= Version::new(0, 0, 4) {
        asc_get::<_, AscUnresolvedContractCallV4, _>(&fenv, asc_ptr.into(), 0)?
    } else {
        asc_get::<_, AscUnresolvedContractCall, _>(&fenv, asc_ptr.into(), 0)?
    };
    let env = fenv.data_mut();
    let request = CallRequest::EthereumContractCall(call);
    let result = env.rpc.handle_request(request);

    match result {
        Ok(CallResponse::EthereumContractCall(tokens)) => {
            let asc_result = asc_new(&mut fenv, tokens.as_slice())?;
            Ok(asc_result)
        }
        Err(_) => Ok(AscPtr::null()),
    }
}

#[cfg(test)]
mod test {
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::chain::ethereum::event::EthereumEventData;
    use crate::chain::ethereum::transaction::EthereumTransactionData;
    use crate::host_fn_test;
    use crate::rpc_client::tests::create_rpc_client_test;
    use crate::runtime::asc::base::asc_get;
    use crate::runtime::asc::base::asc_new;
    use crate::runtime::asc::base::AscPtr;
    use crate::runtime::asc::native_types::string::AscString;
    use crate::runtime::wasm_host::test::*;
    use df_logger::loggers;
    use df_logger::loggers::init_logger;
    use ethabi::ethereum_types::H256;
    use ethabi::ethereum_types::U64;
    use std::str::FromStr;
    use wasmer::Value;
    use web3::types::Address;

    host_fn_test!("TestTypes", test_ethereum_block, host, result {
        let block = EthereumBlockData {
            number: U64::from(153453),
            hash: H256::from_str("0xfe52a399d93c48b67bb147432aff55873576997d9d05de2c97087027609ae440")
                .unwrap(),
            ..Default::default()
        };
        let asc_block = asc_new(&mut host, &block).unwrap();
        let block_ptr = asc_block.wasm_ptr() as i32;

        let tx = EthereumTransactionData {
            hash: H256::from_str("0x65077e1060e4d159d053afd8f3edc6fd1f56a06b94aab2987607e6850c9d5af4").unwrap(),
            ..Default::default()
        };
        let asc_tx = asc_new(&mut host, &tx).unwrap();
        let tx_ptr = asc_tx.wasm_ptr() as i32;

        let event = EthereumEventData {
            address: Address::from_str("0x388c818ca8b9251b393131c08a736a67ccb19297").unwrap(),
            ..Default::default()
        };
        let asc_event = asc_new(&mut host, &event).unwrap();
        let event_ptr = asc_event.wasm_ptr() as i32;

        [Value::I32(block_ptr), Value::I32(tx_ptr), Value::I32(event_ptr)]
    } {
        let asc_str = AscPtr::<AscString>::new(result.first().unwrap().unwrap_i32() as u32);
        let returned_str: String = asc_get(&host, asc_str, 0).unwrap();
        let expected_str = "block_number=153453, tx_hash=0x65077e1060e4d159d053afd8f3edc6fd1f56a06b94aab2987607e6850c9d5af4, event_address=0x388c818ca8b9251b393131c08a736a67ccb19297";
        assert_eq!(returned_str, expected_str);
    });

    #[rstest::rstest]
    #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_ethereum_call(#[case] version: &str) {
        use std::env;

        use prometheus::default_registry;
        let registry = default_registry();

        env::set_var("SUBGRAPH_WASM_RUNTIME_TEST", "YES");
        init_logger();

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (version, wasm_path) =
                    get_subgraph_testing_resource(version, "TestEthereumCall");
                let rpc = create_rpc_client_test(&version.to_string().replace('.', "_")).await;

                let mut host = mock_wasm_host(version, &wasm_path, registry, rpc);
                let func = host
                    .instance
                    .exports
                    .get_function("testEthereumCall")
                    .unwrap();

                func.call(&mut host.store, &[])
                    .expect("Calling function failed!");
            });
    }
}
