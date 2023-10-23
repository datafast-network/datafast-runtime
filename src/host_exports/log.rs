use log;
use wasmer::{FunctionEnvMut, FunctionType, Store, Type, WasmError};

use crate::asc::{
    base::{AscType, FromAscObj},
    native_types::string::AscString,
};

pub fn log_log(log_level: i32, message: i32) -> Result<(), WasmError> {
    /// How to access store / memory
    /// memory.view()[ptr] -> real data
    let data = data.to_be_bytes();
    let asc_string = AscString::from_asc_bytes(&data).unwrap();

    Ok(())
}

pub fn sum(_env: FunctionEnvMut<()>, a: i32, b: i32) -> i32 {
    log::info!("{a}, {b}");
    a + b
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
