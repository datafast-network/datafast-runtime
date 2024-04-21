use log::{error, info};
use wasmer::{FunctionEnvMut, RuntimeError};
use crate::runtime::asc::base::{asc_get, asc_new, AscPtr};
use crate::runtime::asc::native_types::{Uint8Array};
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::wasm_host::Env;

pub fn store_set(
    mut fenv: FunctionEnvMut<Env>,
    key: AscPtr<AscString>,
    bytes_ptr: AscPtr<Uint8Array>,
) -> Result<(), RuntimeError> {
    let key = asc_get::<String, _, _>(&fenv, key, 0)?;
    let bytes = asc_get::<Vec<u8>, _, _>(&fenv, bytes_ptr, 0)?;

    let env = fenv.data_mut();
    if env.store_filter.is_none() {
        error!("store_filter is not initialized");
        return Err(RuntimeError::new(
            "store_filter is not initialized".to_string(),
        ));
    }
    let store = env.store_filter.as_ref().unwrap();
    info!("store_set key: {:?}", key);
    match store.set(&key, bytes) {
        Ok(_) => {
            info!("store_filter_set success");
            Ok(())
        }
        Err(e) => {
            info!("store_filter_set failed: {:?}", e);
            Err(RuntimeError::new(
                "store_filter_set failed".to_string(),
            ))
        }
    }
}

pub fn store_get(
    mut fenv: FunctionEnvMut<Env>,
    key_ptr: AscPtr<AscString>,
) -> Result<AscPtr<Uint8Array>, RuntimeError> {
    let key = asc_get::<String, _, _>(&fenv, key_ptr, 0)?;
    let env = fenv.data();
    if env.store_filter.is_none() {
        error!("store_filter is not initialized");
        return Err(RuntimeError::new(
            "store_filter is not initialized".to_string(),
        ));
    }
    let store = env.store_filter.as_ref().unwrap();

    info!("store_get key: {:?}", key);
    match store.get(&key) {
        Ok(bytes) => {
            info!("store_filter_get success");
            let data = bytes.as_slice();
            let bytes_ptr = asc_new(&mut fenv, data)?;
            Ok(bytes_ptr)
        }
        Err(e) => {
            info!("store_filter_get failed: {:?}", e);
            Err(RuntimeError::new(
                "store_filter_get failed".to_string(),
            ))
        }
    }
}

pub fn store_remove(
    fenv: FunctionEnvMut<Env>,
    key_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let env = fenv.data();
    if env.store_filter.is_none() {
        error!("store_filter is not initialized");
        return Err(RuntimeError::new(
            "store_filter is not initialized".to_string(),
        ));
    }
    let store = env.store_filter.as_ref().unwrap();
    let key = asc_get::<String, _, _>(&fenv, key_ptr, 0)?;
    match store.remove(&key) {
        Ok(_) => {
            info!("store_filter_remove success");
            Ok(())
        }
        Err(e) => {
            info!("store_filter_remove failed: {:?}", e);
            Err(RuntimeError::new(
                "store_filter_remove failed".to_string(),
            ))
        }
    }
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
        let mut host = mock_wasm_host(version, ws_path, registry, RpcAgent::new_mock(registry), None);
        let func = host
            .instance
            .exports
            .get_function("TestFilter")
            .expect("No function with name `getWhiteListAddress` exists!");
        let result = func.call(&mut host.store, &[]).unwrap_or_default();
    }
}