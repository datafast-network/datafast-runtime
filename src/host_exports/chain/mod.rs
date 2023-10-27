#[cfg(test)]
mod test {
    use crate::asc::base::asc_new;
    use crate::chain::ethereum::block::EthereumBlockData;
    use crate::host_exports::test::*;
    use crate::host_fn_test;
    use ethabi::ethereum_types::H256;
    use ethabi::ethereum_types::U64;
    use std::str::FromStr;
    use wasmer::Value;

    host_fn_test!(
        test_ethereum_block,
        host,
        _void
        {
            let mut block = EthereumBlockData::default();
            block.number = U64::from_str_radix("153453", 10).unwrap();
            block.hash =
                H256::from_str("0xfe52a399d93c48b67bb147432aff55873576997d9d05de2c97087027609ae440")
                .unwrap();
            let asc_block = asc_new(&mut host, &mut block).unwrap();
            let ptr = asc_block.wasm_ptr() as i32;
            [Value::I32(ptr)]
        }
        {}
    );
}
