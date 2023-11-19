use crate::chain::ethereum::asc::EthereumValueKind;
use crate::errors::AscError;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::r#enum::AscEnum;
use crate::runtime::asc::native_types::r#enum::AscEnumArray;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::asc::native_types::Uint8Array;
use crate::runtime::wasm_host::Env;
use wasmer::FunctionEnvMut;

pub fn ethereum_encode(
    _fenv: FunctionEnvMut<Env>,
    _token: AscPtr<AscEnum<EthereumValueKind>>,
) -> Result<AscPtr<Uint8Array>, AscError> {
    todo!()
}

pub fn ethereum_decode(
    _fenv: FunctionEnvMut<Env>,
    _types_ptr: AscPtr<AscString>,
    _data_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscEnum<EthereumValueKind>>, AscError> {
    todo!()
}

pub fn crypto_keccak_256(
    _fenv: FunctionEnvMut<Env>,
    _input_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<Uint8Array>, AscError> {
    todo!()
}

pub fn ethereum_call(
    _fenv: FunctionEnvMut<Env>,
    _wasm_ptr: i32,
) -> Result<AscEnumArray<EthereumValueKind>, AscError> {
    todo!()
}

#[cfg(test)]
mod test {
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::chain::ethereum::event::EthereumEventData;
    use crate::chain::ethereum::transaction::EthereumTransactionData;
    use crate::host_fn_test;
    use crate::runtime::asc::base::asc_get;
    use crate::runtime::asc::base::asc_new;
    use crate::runtime::asc::base::AscPtr;
    use crate::runtime::asc::native_types::string::AscString;
    use crate::runtime::wasm_host::test::*;
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
}
