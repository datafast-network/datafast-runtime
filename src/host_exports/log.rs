use super::Env;
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
    // NOTE: this implementation is very verbose, therefore only good for demonstration purpose
    // and need refactoring in the future
    // TODO: refactor to a more generic approach
    // msg_ptr should become WasmPtr<String> or AscPtr<String>
    let store_ref = fenv.as_store_ref();
    let env = fenv.data();
    let api_version = env.api_version.clone();

    let memory = &env.memory.clone().unwrap();
    let view = memory.view(&store_ref);

    let size = view.data_size();
    let capacity = (size - msg_ptr as u64) as usize;

    // NOTE: We accquire full data of the page's remaining (pointer location -> end of page)
    let mut buf = vec![0; capacity];
    view.read(msg_ptr as u64, &mut buf).unwrap();

    let asc_string = AscString::from_asc_bytes(&buf, &api_version).unwrap();
    let mut string = String::from_utf16(asc_string.content()).unwrap();

    // Strip null characters
    if string.contains('\u{0000}') {
        string = string.replace('\u{0000}', "");
    }

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
