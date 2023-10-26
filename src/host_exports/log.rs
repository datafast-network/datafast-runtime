use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::native_types::string::AscString;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    fenv: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let string: String = asc_get(&fenv, msg_ptr, 0)?;
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

    #[test]
    fn test_log() {
        ::env_logger::try_init().unwrap_or_default();
        let test_wasm_file_path = env::var("TEST_WASM_FILE").expect("Test Wasm file not found");
        log::info!("Test Wasm path: {test_wasm_file_path}");
        let (mut store, instance) = create_mock_host_instance(&test_wasm_file_path).unwrap();
        let f = instance.exports.get_function("testLog").unwrap();
        log::info!("-- calling");
        f.call(&mut store, &[]).unwrap();
    }
}
