use wasmer::{FunctionEnvMut, RuntimeError};
use crate::runtime::asc::base::{asc_new, AscPtr};
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::wasm_host::Env;

pub fn get_white_list_address(
    mut fenv: FunctionEnvMut<Env>,
) -> Result<AscPtr<Array<AscPtr<AscString>>>, RuntimeError> {
    let _env = fenv.data_mut();
    let list = vec!["address1", "address2", "address3"];
    let list_ptr = asc_new(&mut fenv, list.as_slice())?;
    Ok(list_ptr)
}

#[cfg(test)]
mod test {
    use prometheus::default_registry;
    use semver::Version;
    use crate::rpc_client::RpcAgent;
    use crate::runtime::wasm_host::test::mock_wasm_host;

    #[tokio::test]
    async fn test_get_white_list_address() {
        env_logger::try_init().unwrap_or_default();
        let version = Version::parse("0.0.5").unwrap();
        let ws_path = "../subgraph-testing/packages/uniswap-v3/build/Datafilter/Datafilter.wasm";
        let registry = default_registry();
        let mut host = mock_wasm_host(version, ws_path, registry, RpcAgent::new_mock(registry));
        let func = host
            .instance
            .exports
            .get_function("TestFilter")
            .expect("No function with name `getWhiteListAddress` exists!");
        let result = func.call(&mut host.store, &[]).unwrap_or_default();
    }
}