mod asc;
mod bigdecimal;
mod bigint;
mod chain;
mod datasource;
mod global;
mod json;
mod macros;
mod store;
mod types_conversion;
mod wasm_log;

use crate::common::BlockPtr;
use crate::components::ManifestAgent;
use crate::database::DatabaseAgent;
use crate::errors::WasmHostError;
use crate::rpc_client::RpcAgent;
pub use asc::AscHost;
use semver::Version;
use wasmer::imports;
use wasmer::Function;
use wasmer::FunctionEnv;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::Module;
use wasmer::Store;
use wasmer::TypedFunction;

#[derive(Clone)]
pub struct Env {
    pub memory: Option<Memory>,
    pub memory_allocate: Option<TypedFunction<i32, i32>>,
    pub api_version: Version,
    pub id_of_type: Option<TypedFunction<u32, u32>>,
    pub arena_start_ptr: i32,
    pub db_agent: DatabaseAgent,
    pub datasource_name: String,
    pub datasource_network: String,
    pub datasource_address: Option<String>,
    pub rpc_agent: RpcAgent,
    pub manifest_agent: ManifestAgent,
    pub block_ptr: BlockPtr,
}

pub fn create_wasm_host(
    api_version: Version,
    wasm_bytes: Vec<u8>,
    db_agent: DatabaseAgent,
    datasource_name: String,
    rpc_agent: RpcAgent,
    manifest_agent: ManifestAgent,
    datasource_address: Option<String>,
    block_ptr: BlockPtr,
    datasource_network: String,
) -> Result<AscHost, WasmHostError> {
    let mut store = Store::default();
    let module = Module::new(&store, wasm_bytes)?;

    let env = FunctionEnv::new(
        &mut store,
        Env {
            memory: None,
            memory_allocate: None,
            id_of_type: None,
            api_version: api_version.clone(),
            arena_start_ptr: 0,
            db_agent: db_agent.clone(),
            datasource_name,
            rpc_agent: rpc_agent.clone(),
            manifest_agent,
            datasource_address,
            block_ptr,
            datasource_network,
        },
    );

    // Running cargo-run will immediately tell which functions are missing
    let import_object = imports! {
        "env" => {
            "abort" => Function::new_typed_with_env(&mut store, &env, global::abort)
        },
        "conversion" => {
            "typeConversion.bytesToString" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_string),
            "typeConversion.bytesToHex" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_hex),
            "typeConversion.bigIntToString" => Function::new_typed_with_env(&mut store, &env, types_conversion::big_int_to_string),
            "typeConversion.bigIntToHex" => Function::new_typed_with_env(&mut store, &env, types_conversion::big_int_to_hex),
            "typeConversion.stringToH160" => Function::new_typed_with_env(&mut store, &env, types_conversion::string_to_h160),
            "typeConversion.bytesToBase58" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_base58),
        },
        "numbers" => {
            "bigInt.plus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_plus),
            "bigInt.minus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_minus),
            "bigInt.times" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_times),
            "bigInt.dividedBy" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_divided_by),
            "bigInt.dividedByDecimal" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_divided_by_decimal),
            "bigInt.pow" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_pow),
            "bigInt.mod" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_mod),
            "bigInt.fromString" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_from_string),
            "bigInt.bitOr" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_bit_or),
            "bigInt.bitAnd" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_bit_and),
            "bigInt.leftShift" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_left_shift),
            "bigInt.rightShift" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_right_shift),
            //Big Decimal
            "bigDecimal.fromString" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_from_string),
            "bigDecimal.toString" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_to_string),
            "bigDecimal.plus" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_plus),
            "bigDecimal.minus" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_minus),
            "bigDecimal.times" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_times),
            "bigDecimal.dividedBy" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_divided_by),
            "bigDecimal.equals" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_equals),
        },
        "json" => {
            "json.toBigInt" =>Function::new_typed_with_env(&mut store, &env, json::json_to_bigint),
        },
        "ethereum" => {
            //Ethereum fn
            "ethereum.encode" =>  Function::new_typed_with_env(&mut store, &env, chain::ethereum::ethereum_encode),
            "ethereum.decode" =>  Function::new_typed_with_env(&mut store, &env, chain::ethereum::ethereum_decode),
            "ethereum.call" =>  Function::new_typed_with_env(&mut store, &env, chain::ethereum::ethereum_call),
            "crypto.keccak256" => Function::new_typed_with_env(&mut store, &env, chain::ethereum::crypto_keccak_256),
        },
        "datasource" => {
            // Datasource
            "dataSource.create" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_create),
            "dataSource.createWithContext" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_create_context),
            "dataSource.address" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_address),
            "dataSource.network" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_network),
            "dataSource.context" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_context)
        },
        "index" => { //index for subgraph version <= 4
            "store.set" => Function::new_typed_with_env(&mut store, &env, store::store_set),
            "store.get" => Function::new_typed_with_env(&mut store, &env, store::store_get),
            "store.remove" => Function::new_typed_with_env(&mut store, &env, store::store_remove),
            "store.loadRelated" => Function::new_typed_with_env(&mut store, &env, store::store_load_related),
            "store.get_in_block" => Function::new_typed_with_env(&mut store, &env, store::store_get_in_block),
            //Convert
            "typeConversion.bytesToString" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_string),
            "typeConversion.bytesToHex" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_hex),
            "typeConversion.bigIntToString" => Function::new_typed_with_env(&mut store, &env, types_conversion::big_int_to_string),
            "typeConversion.bigIntToHex" => Function::new_typed_with_env(&mut store, &env, types_conversion::big_int_to_hex),
            "typeConversion.stringToH160" => Function::new_typed_with_env(&mut store, &env, types_conversion::string_to_h160),
            "typeConversion.bytesToBase58" => Function::new_typed_with_env(&mut store, &env, types_conversion::bytes_to_base58),
            //Log
            "log.log" => Function::new_typed_with_env(&mut store, &env, wasm_log::log_log),
            // Datasource
            "dataSource.create" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_create),
            "dataSource.createWithContext" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_create_context),
            "dataSource.address" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_address),
            "dataSource.network" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_network),
            "dataSource.context" => Function::new_typed_with_env(&mut store, &env, datasource::datasource_context),
            // BigInt
            "bigInt.plus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_plus),
            "bigInt.minus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_minus),
            "bigInt.minus" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_minus),
            "bigInt.times" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_times),
            "bigInt.dividedBy" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_divided_by),
            "bigInt.dividedByDecimal" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_divided_by_decimal),
            "bigInt.pow" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_pow),
            "bigInt.mod" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_mod),
            "bigInt.fromString" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_from_string),
            "bigInt.bitOr" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_bit_or),
            "bigInt.bitAnd" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_bit_and),
            "bigInt.leftShift" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_left_shift),
            "bigInt.rightShift" => Function::new_typed_with_env(&mut store, &env, bigint::big_int_right_shift),
            //Big Decimal
            "bigDecimal.fromString" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_from_string),
            "bigDecimal.toString" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_to_string),
            "bigDecimal.plus" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_plus),
            "bigDecimal.minus" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_minus),
            "bigDecimal.times" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_times),
            "bigDecimal.dividedBy" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_divided_by),
            "bigDecimal.equals" => Function::new_typed_with_env(&mut store, &env, bigdecimal::big_decimal_equals),
        }
    };

    // Running cargo-run will immediately tell which functions are missing
    let instance = Instance::new(&mut store, &module, &import_object)?;

    // Bind guest memory ref & __alloc to env
    let mut env_mut = env.into_mut(&mut store);
    let (data_mut, mut store_mut) = env_mut.data_and_store_mut();

    data_mut.memory = Some(
        instance
            .exports
            .get_memory("memory")
            // NOTE: This is default memory of WASMER, so it should basically never fail
            .expect("No global memory function")
            .clone(),
    );
    assert!(data_mut.memory.is_some(), "Global Memory set");

    data_mut.memory_allocate = match api_version.clone() {
        version if version <= Version::new(0, 0, 4) => instance
            .exports
            .get_typed_function(&store_mut, "memory.allocate")
            .ok(),
        _ => instance
            .exports
            .get_typed_function(&store_mut, "allocate")
            .ok(),
    };

    data_mut.id_of_type = match api_version.clone() {
        version if version <= Version::new(0, 0, 4) => None,
        _ => instance
            .exports
            .get_typed_function(&store_mut, "id_of_type")
            .ok(),
    };

    match data_mut.api_version.clone() {
        version if version <= Version::new(0, 0, 4) => {}
        _ => {
            instance
                .exports
                .get_function("_start")
                .map(|f| {
                    f.call(&mut store_mut, &[]).unwrap();
                })
                .ok();
        }
    }

    let memory = instance.exports.get_memory("memory").unwrap().clone();
    let id_of_type = data_mut.id_of_type.clone();
    let arena_start_ptr = data_mut.arena_start_ptr.clone();
    let memory_allocate = data_mut.memory_allocate.clone();

    let host = AscHost {
        store,
        instance,
        api_version,
        memory,
        memory_allocate,
        id_of_type,
        arena_start_ptr,
        db_agent,
        rpc_agent,
    };

    Ok(host)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use prometheus::Registry;
    use std::path::PathBuf;

    pub fn mock_wasm_host(
        api_version: Version,
        wasm_path: &str,
        registry: &Registry,
        rpc_agent: RpcAgent,
    ) -> AscHost {
        ::log::warn!(
            r#"New test-host-instance being created with:
                > api-version={api_version}
                > wasm-file-path={wasm_path}
            "#
        );

        let wasm_bytes = std::fs::read(wasm_path).expect("Bad wasm file, cannot load");
        let db_agent = DatabaseAgent::empty(registry);

        create_wasm_host(
            api_version,
            wasm_bytes,
            db_agent,
            "Test".to_string(),
            rpc_agent,
            ManifestAgent::default(),
            None,
            BlockPtr::default(),
            "test".to_string(),
        )
        .unwrap()
    }

    pub fn get_subgraph_testing_resource(
        version: &str,
        datasource_name: &str,
    ) -> (Version, String) {
        let version = Version::parse(version).expect("Bad api-version");
        let mut project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let version_as_package_dir = version.to_string().replace('.', "_");
        project_path.push(format!(
            "../subgraph-testing/packages/v{version_as_package_dir}/build/{datasource_name}/{datasource_name}.wasm"
        ));
        let wasm_path = project_path.into_os_string().into_string().unwrap();

        (version, wasm_path)
    }
}
