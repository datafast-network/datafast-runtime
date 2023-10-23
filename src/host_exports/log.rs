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
    }
}
