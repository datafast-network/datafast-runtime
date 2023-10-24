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
    let store_ref = fenv.as_store_ref();
    let env = fenv.data();
    let memory = &env.memory.clone().unwrap();
    let view = memory.view(&store_ref);

    let mut buf = [0u8; 1024];
    view.read(msg_ptr as u64, &mut buf).unwrap();

    let asc_string = AscString::from_asc_bytes(&buf).unwrap();
    let mut string = String::from_utf16(asc_string.content()).unwrap();

    // Strip null characters
    if string.contains('\u{0000}') {
        string = string.replace('\u{0000}', "");
    }

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
    use super::super::create_host_instance;
    use env_logger;

    #[test]
    fn test_log() {
        env_logger::try_init().unwrap_or_default();
        let (mut store, instance) = create_host_instance(
            "/Users/vutran/Works/hardbed/subgraph-wasm-runtime/src/host_exports/test_log.wasm",
        )
        .unwrap();
        let f = instance.exports.get_function("myown").unwrap();
        log::info!("-- calling");
        f.call(&mut store, &[]).unwrap();
    }
}
