mod asc;
mod bigint;
mod log;

use semver::Version;
use wasmer::Memory;
use wasmer::TypedFunction;

#[derive(Clone)]
pub struct Env {
    pub memory: Option<Memory>,
    pub alloc_guest_memory: Option<TypedFunction<i32, i32>>,
    pub api_version: Version,
    pub id_of_type: Option<TypedFunction<i32, i32>>,
}

#[cfg(test)]
mod test {
    use super::bigint;
    use super::log as host_log;
    use super::Env;
    use crate::conversion;
    use crate::global;
    use crate::store;
    use log;
    use semver::Version;
    use std::env;
    use wasmer::imports;
    use wasmer::Function;
    use wasmer::FunctionEnv;
    use wasmer::Instance;
    use wasmer::Module;
    use wasmer::Store;
    use wasmer::TypedFunction;

    pub fn create_mock_host_instance(
        wasm_path: &str,
    ) -> Result<(Store, Instance), Box<dyn std::error::Error>> {
        let wasm_bytes = std::fs::read(wasm_path)?;
        let mut store = Store::default();

        let module = Module::new(&store, wasm_bytes)?;
        let api_version = Version::parse(
            env::var("RUNTIME_API_VERSION")
                .unwrap_or("0.0.5".to_string())
                .as_str(),
        )
        .unwrap();
        log::warn!("________________________ Init WASM Instance with api-version={api_version}");
        let env = FunctionEnv::new(
            &mut store,
            Env {
                memory: None,
                alloc_guest_memory: None,
                api_version,
                id_of_type: None,
            },
        );

        // Global
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
                "abort" => abort,
            },
            "conversion" => {
                "typeConversion.bigIntToHex" => big_int_to_hex,
                "typeConversion.bytesToHex" => bytes_to_hex,
                "typeConversion.bigIntToString" => big_int_to_string,
            },
            "numbers" => {
                "bigDecimal.toString" => big_decimal_to_string.clone(),
                "bigInt.plus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_plus)
            },
            "index" => {
                "store.set" => store_set,
                "store.get" => store_get,
                "log.log" => Function::new_typed_with_env(&mut store, &env, host_log::log_log),
                "bigInt.plus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_plus)
            }
        };
        let instance = Instance::new(&mut store, &module, &import_object)?;

        // Bind guest memory ref & __alloc to env
        let mut env_mut = env.into_mut(&mut store);
        let (data_mut, mut store_mut) = env_mut.data_and_store_mut();

        data_mut.memory = Some(instance.exports.get_memory("memory")?.clone());
        let alloc_guest_memory: Option<TypedFunction<i32, i32>> = instance
            .exports
            .get_typed_function(&mut store_mut, "__alloc")
            // NOTE: depend on the mapping logic, this might or might not be exported
            .ok();

        if let Some(guest_alloc) = alloc_guest_memory {
            guest_alloc.call(&mut store_mut, 0).unwrap();
            data_mut.alloc_guest_memory = Some(guest_alloc);
        };

        data_mut.id_of_type = instance
            .exports
            .get_typed_function(&store_mut, "id_of_type")
            // NOTE: depend on the mapping logic, this might or might not be exported
            .ok();

        match data_mut.api_version.clone() {
            version if version <= Version::new(0, 0, 4) => {}
            _ => {
                log::warn!("Try calling `_start` if possible");
                instance
                    .exports
                    .get_function("_start")
                    .map(|f| {
                        log::warn!("Calling `_start`");
                        f.call(&mut store_mut, &[]).unwrap();
                    })
                    .ok();
            }
        }

        Ok((store, instance))
    }
}
