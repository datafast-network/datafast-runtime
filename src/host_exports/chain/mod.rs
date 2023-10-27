#[cfg(test)]
mod test {
    use crate::asc::base::asc_new;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::host_exports::test::*;
    use ethabi::ethereum_types::H256;
    use ethabi::ethereum_types::U64;
    use std::str::FromStr;
    use wasmer::Value;

    #[::rstest::rstest]
    // #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_eth_block(#[case] version: &str) {
        use env_logger;

        env_logger::try_init().unwrap_or_default();
        let (version, wasm_path) = version_to_test_resource(version);

        let mut host = mock_host_instance(version, &wasm_path);
        let mut block = EthereumBlockData::default();
        block.number = U64::from_str_radix("153453", 10).unwrap();
        block.hash =
            H256::from_str("0xfe52a399d93c48b67bb147432aff55873576997d9d05de2c97087027609ae440")
                .unwrap();
        let asc_block = asc_new(&mut host, &mut block).unwrap();

        let ptr = asc_block.wasm_ptr();
        let func = host
            .instance
            .exports
            .get_function("testEthereumBlock")
            .unwrap();

        let result = func
            .call(&mut host.store, &[Value::I32(ptr as i32)])
            .expect("Calling function failed!");
        assert!(result.is_empty());
    }
}
