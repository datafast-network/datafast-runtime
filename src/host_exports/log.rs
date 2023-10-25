use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::native_types::string::AscString;
use wasmer::AsStoreRef;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: i32,
) -> Result<(), RuntimeError> {
    let asc_string = AscPtr::<AscString>::from(msg_ptr as u32);
    let string: String = asc_get(&fenv, asc_string, 0).unwrap();
    // TODO: we use simple logging for now, but a dedicated logger must be setup for each wasm instance
    match log_level {
        0 => eprintln!("CRITICAL!!!!!!: {string}"),
        1 => log::error!("{string}"),
        2 => log::warn!("{string}"),
        3 => log::info!("{string}"),
        4 => log::debug!("{string}"),
        _ => return Err(RuntimeError::new("Invalid log level!!")),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::super::test::create_mock_host_instance;
    use std::env;
    use wasmer::AsStoreMut;

    #[test]
    fn test_log() {
        ::env_logger::try_init().unwrap_or_default();

        let test_wasm_file_path = env::var("TEST_WASM_FILE").expect("Test Wasm file not found");
        let (mut store, instance) = create_mock_host_instance(&test_wasm_file_path).unwrap();
        let f = instance.exports.get_function("testLog").unwrap();
        log::info!("-- calling");
        f.call(&mut store.as_store_mut(), &[]).unwrap();
    }
}
