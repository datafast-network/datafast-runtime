use super::Env;

use log::info;
use wasmer::AsStoreRef;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn log_log(
    f_env: FunctionEnvMut<Env>,
    log_level: i32,
    msg_ptr: i32,
) -> Result<(), RuntimeError> {
    log::info!("{log_level}, ptr={msg_ptr}");

    let store_ref = f_env.as_store_ref();
    let inner_env = f_env.data();
    let memory = &inner_env.memory;

    let view = memory.view(&store_ref);
    let data = view.copy_to_vec().unwrap();
    let find = data.iter().find(|slot| **slot > 0);
    info!("data = {:?}, length={}", find, data.len());
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::asc::{base::AscType, native_types::string::AscString};

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
        let ptr = f.call(&mut store, &[]).unwrap();
        log::info!("{:?}", ptr);

        let memory = instance.exports.get_memory("memory").unwrap();
        let view = memory.view(&store);
        let guest_data = view.copy_to_vec().unwrap();
        let find = guest_data.iter().find(|slot| **slot > 0);
        log::info!("data = {:?}, length={}", find, guest_data.len());

        // let mut buf = Vec::new();
        // let buf = buf.as_mut_slice();
        let mut buf = [0; 128];
        view.read(14852, &mut buf).unwrap();
        let asc_str = AscString::from_asc_bytes(&buf).unwrap();
        let content = String::from_utf16(asc_str.content()).unwrap();
        log::info!("{}", content);
    }
}
