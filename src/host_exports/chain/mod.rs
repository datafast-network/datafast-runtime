#[cfg(test)]
mod test {
    use wasmer::Value;

    use super::super::asc::*;
    use crate::asc::base::asc_new;
    use crate::asc::base::ToAscObj;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::host_exports::test::*;

    #[::rstest::rstest]
    // #[case("0.0.4")]
    #[case("0.0.5")]
    fn test_eth_block(#[case] version: &str) {
        use env_logger;

        env_logger::try_init().unwrap_or_default();
        let (version, wasm_path) = version_to_test_resource(version);

        let mut host = mock_host_instance(version, &wasm_path);
        let mut block = EthereumBlockData::default();
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
