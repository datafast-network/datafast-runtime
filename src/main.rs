use wasmer::imports;
use wasmer::Function;
use wasmer::FunctionEnv;
use wasmer::FunctionType;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
use wasmer::TypedFunction;
use wasmer::Value;

struct Env {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasm_bytes = std::fs::read("./subgraph.wasm")?;
    let mut store = Store::default();
    let module = Module::new(&store, wasm_bytes)?;

    // Define env of host
    let env = FunctionEnv::new(&mut store, Env {});

    // Define host functions
    let abort_type = FunctionType::new(vec![], vec![]);
    let abort = Function::new(&mut store, &abort_type, |_| Ok(vec![Value::I32(0)]));

    // Running cargo-run will immediately tell which functions are missing
    let import_object = imports! {
        "env" => {
            "abort" => abort
        }
    };
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let handle_gravatar: TypedFunction<i32, i32> = instance
        .exports
        .get_function("handleNewGravatar")?
        .typed(&mut store)?;

    println!("Calling `handle_gravatar` function...");
    let result = handle_gravatar.call(&mut store, 1)?;

    println!("Results of `handle_gravatar`: {:?}", result);
    assert_eq!(result, 2);

    Ok(())
}
