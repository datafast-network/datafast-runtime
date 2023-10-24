mod asc;
mod bigint;
mod log;
mod type_conversion;

use wasmer::Memory;
use wasmer::TypedFunction;

#[derive(Clone)]
pub struct Env {
    pub memory: Option<Memory>,
    pub alloc_guest_memory: Option<TypedFunction<i32, i32>>,
    id_of_type: Option<TypedFunction<i32, i32>>,
}

#[cfg(test)]
mod test {
    use super::bigint;
    use super::log;
    use super::type_conversion;
    use super::Env;
    use crate::global;
    use crate::store;
    use wasmer::imports;
    use wasmer::Function;
    use wasmer::FunctionEnv;
    use wasmer::FunctionEnvMut;
    use wasmer::Instance;
    use wasmer::Module;
    use wasmer::Store;

    pub fn create_mock_host_instance(
        wasm_path: &str,
    ) -> Result<(Instance, FunctionEnvMut<Env>), Box<dyn std::error::Error>> {
        let wasm_bytes = std::fs::read(wasm_path)?;
        let mut store = Store::default();

        let module = Module::new(&store, wasm_bytes)?;
        let fenv = FunctionEnv::new(
            &mut store,
            Env {
                memory: None,
                alloc_guest_memory: None,
                id_of_type: None,
            },
        );

        // Global
        let abort = Function::new(&mut store, global::ABORT_TYPE, global::abort);

        // Conversion functions
        let big_int_to_hex = Function::new(
            &mut store,
            type_conversion::CONVERSION_TYPE,
            // TODO: fix implementation
            type_conversion::big_int_to_hex,
        );

        let big_decimal_to_string = Function::new(
            &mut store,
            type_conversion::CONVERSION_TYPE,
            // TODO: fix implementation
            type_conversion::big_int_to_hex,
        );

        let bytes_to_hex = Function::new(
            &mut store,
            type_conversion::CONVERSION_TYPE,
            // TODO: fix implementation
            type_conversion::bytes_to_hex,
        );

        let big_int_to_string = Function::new(
            &mut store,
            type_conversion::CONVERSION_TYPE,
            // TODO: fix implementation
            type_conversion::big_int_to_string,
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
                "abort" => abort,
            },
            "conversion" => {
                "typeConversion.bigIntToHex" => big_int_to_hex,
                "typeConversion.bytesToHex" => bytes_to_hex,
                "typeConversion.bigIntToString" => big_int_to_string,
            },
            "numbers" => {
                "bigDecimal.toString" => big_decimal_to_string,
                "bigInt.plus" => Function::new_typed_with_env(&mut store, &fenv, bigint::big_int_plus),
            },
            "index" => {
                "store.set" => store_set,
                "store.get" => store_get,
                "log.log" => Function::new_typed_with_env(&mut store, &fenv, log::log_log),
            }
        };
        let instance = Instance::new(&mut store, &module, &import_object)?;

        // Bind guest memory ref & __alloc to env
        let mut env_mut = fenv.into_mut(&mut store);
        let (data_mut, mut store_mut) = env_mut.data_and_store_mut();

        data_mut.memory = Some(instance.exports.get_memory("memory")?.clone());
        data_mut.alloc_guest_memory = instance
            .exports
            .get_typed_function(&mut store_mut, "__alloc")
            // NOTE: depend on the mapping logic, this might or might not be exported
            .ok();
        data_mut.id_of_type = instance
            .exports
            .get_typed_function(&mut store_mut, "id_of_type")
            // NOTE: depend on the mapping logic, this might or might not be exported
            .ok();

        Ok((instance, data_mut))
    }
}
