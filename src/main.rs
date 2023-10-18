mod asc_abi;
mod chain;
mod cheap_clone;
mod conversion;
mod global;
mod graph;
mod runtime;
mod store;
mod utils;
mod wasm_context;

use crate::wasm_context::WasmContext;
use wasmer::imports;
use wasmer::wasmparser::Payload::Version;
use wasmer::Function;
use wasmer::Instance;
use wasmer::IntoBytes;
use wasmer::Module;
use wasmer::Store;
use wasmer::TypedFunction;

struct Env {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasm_bytes = std::fs::read("./subgraph.wasm")?;
    // let wat_bytes = std::fs::read("./subgraph.wat")?;
    // println!("------ wat wat: {:?}", str::from_utf8(&wat_bytes));
    // let wasm_bytes = wat2wasm(&wat_bytes).expect("failed to load").to_vec();
    // println!("-------------- OK");
    let mut store = Store::default();
    let module = Module::new(&store, wasm_bytes)?;
    // println!("-------------- OK2");
    // Define env of host
    // let env = FunctionEnv::new(&mut store, Env {});

    // Global functions
    let abort = Function::new(&mut store, global::ABORT_TYPE, global::abort);

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

    // Running cargo-run will immediately tell which functions are missing
    let import_object = imports! {
        "env" => {
            "abort" => abort
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
        }
    };
    let api_version = semver::Version::new(0, 0, 5); //api version of data source from graph.yaml
    let instance = Instance::new(&mut store, &module, &import_object)?;
    let mut instance_ctx = WasmContext::new(&instance, &store, api_version)?;
    let xxx = instance_ctx
        .get_heap_mut()
        .memory_allocate
        .call(&mut store, 1000)?;
    let handle_gravatar: TypedFunction<i32, ()> = instance
        .exports
        .get_function("handleNewGravatar")?
        .typed(&mut store)?;
    //
    println!("Calling `handle_gravatar` function...");
    let result = handle_gravatar.call(&mut store, 1)?;

    println!("Results of `handle_gravatar`: {:?}", result);
    // assert_eq!(result, 2);

    Ok(())
}
