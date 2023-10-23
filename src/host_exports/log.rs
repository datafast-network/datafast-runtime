use log;
use wasmer::{
    AsStoreMut, AsStoreRef, FunctionEnvMut, FunctionType, Memory, MemoryError, MemoryType,
    RuntimeError, Store, Type, Value, WasmError,
};

use super::Env;
use crate::asc::{
    base::{asc_get, AscType, FromAscObj},
    native_types::string::AscString,
};

pub const LOG_TYPE: ([Type; 2], [Type; 0]) = ([Type::I32, Type::I32], []);

pub fn log_log(mut env: FunctionEnvMut<Env>, args: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    // How to access store / memory
    // memory.view()[ptr] -> real data
    let log_level = args[0].clone();
    let ptr = args[1].clone().i32().unwrap();

    log::info!("{:?}", args);

    let mut store_mut = env.as_store_mut();
    let memory = Memory::new(&mut store_mut, MemoryType::new(1, Some(1), false)).unwrap();

    let store_ref = env.as_store_ref();
    let view = memory.view(&store_ref);

    log::info!("Memory size (pages) {:?}", view.size());
    log::info!("Memory size (bytes) {:?}", view.size().bytes());
    log::info!("Memory size (data-size) {:?}", view.data_size());
    // let m = env.memory;

    let mut new_vec = Vec::<u8>::new();
    let mut buf = new_vec.as_mut_slice();
    let str_vec = view.read(0, &mut buf).unwrap();

    // Convert the subslice to a `&str`.
    log::info!("buffer: {:?}", buf);
    let msg = AscString::from_asc_bytes(buf).unwrap();
    let msg = String::from_utf16(msg.content()).unwrap();
    log::info!("message: {msg}");
    Ok(vec![])
}

pub fn sum(_env: FunctionEnvMut<()>, a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod test {
    use super::super::create_host_instance;
    use super::*;
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
    }
}
