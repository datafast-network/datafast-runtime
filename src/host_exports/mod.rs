pub mod log;

use wasmer::imports;
use wasmer::Function;
use wasmer::FunctionEnv;
use wasmer::FunctionEnvMut;
use wasmer::FunctionType;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::MemoryType;
use wasmer::Module;
use wasmer::Store;
use wasmer::Type;

use crate::conversion;
use crate::global;
use crate::store;

pub struct Env {
    pub memory: Memory,
}

fn sum(_env: FunctionEnvMut<()>, a: i32, b: i32) -> i32 {
    a + b
}

pub fn create_host_instance(
    wasm_path: &str,
) -> Result<(Store, Instance), Box<dyn std::error::Error>> {
    let wasm_bytes = std::fs::read(wasm_path)?;
    let mut store = Store::default();
    let memory = Memory::new(&mut store, MemoryType::new(1, None, false)).unwrap();
    let module = Module::new(&store, wasm_bytes)?;

    // Global
    let abort = Function::new(&mut store, global::ABORT_TYPE, global::abort);

    let env = FunctionEnv::new(&mut store, Env { memory });
    // Conversion functions
    let big_int_to_hex = Function::new(
        &mut store,
        conversion::CONVERSION_TYPE,
        // TODO: fix implementation
        conversion::big_int_to_hex,
    );

    let big_decimal_to_string = Function::new(
        &mut store,
        conversion::CONVERSION_TYPE,
        // TODO: fix implementation
        conversion::big_int_to_hex,
    );

    let bytes_to_hex = Function::new(
        &mut store,
        conversion::CONVERSION_TYPE,
        // TODO: fix implementation
        conversion::bytes_to_hex,
    );

    let big_int_to_string = Function::new(
        &mut store,
        conversion::CONVERSION_TYPE,
        // TODO: fix implementation
        conversion::big_int_to_string,
    );

    // Store functions
    let store_set = Function::new(
        &mut store,
        store::STORE_SET_TYPE,
        // TODO: fix implementation
        store::store_set,
    );

    let store_get = Function::new(
        &mut store,
        store::STORE_GET_TYPE,
        // TODO: fix implementation
        store::store_get,
    );

    let memory = Memory::new(&mut store, MemoryType::new(1, None, false)).unwrap();

    // Running cargo-run will immediately tell which functions are missing
    let import_object = imports! {
        "env" => {
            "abort" => abort,
            "memory" => memory
        },
        "conversion" => {
            "typeConversion.bigIntToHex" => big_int_to_hex,
            "typeConversion.bytesToHex" => bytes_to_hex,
            "typeConversion.bigIntToString" => big_int_to_string,
        },
        "numbers" => {
            "bigDecimal.toString" => big_decimal_to_string
        },
        "index" => {
            "store.set" => store_set,
            "store.get" => store_get,
            "log.log" => Function::new_with_env(&mut store, &env, &FunctionType::new(vec![Type::I32, Type::I32], vec![]), log::log_log)
        }
    };
    let instance = Instance::new(&mut store, &module, &import_object)?;

    Ok((store, instance))
}
