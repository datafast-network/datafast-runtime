use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::asc::bignumber::AscBigInt;
use crate::bignumber::bigint::BigInt;
use anyhow::Result;
use wasmer::AsStoreRef;
use wasmer::FunctionEnvMut;
use wasmer::HostFunction;
use wasmer::RuntimeError;
use wasmer::ValueType;
pub fn big_int_plus(mut fenv: FunctionEnvMut<Env>, x: i32, y: i32) -> Result<i32, RuntimeError> {
    let asc_x = AscPtr::<AscBigInt>::new(x as u32);
    let asc_y = AscPtr::<AscBigInt>::new(y as u32);
    let big_int_x: BigInt = asc_get(&fenv, asc_x, 0).unwrap();
    let big_int_y: BigInt = asc_get(&fenv, asc_y, 0).unwrap();
    // log::info!("-- big_int_x: {}", big_int_x.to_string());
    // log::info!("-- big_int_y: {}", big_int_y.to_string());
    let result = big_int_x + big_int_y;
    log::info!("-- result bigint: {}", result.to_string());
    let asc_result = asc_new(&mut fenv, &result).unwrap();
    Ok(asc_result.wasm_ptr() as i32)
}

#[cfg(test)]
mod test {
    use super::super::test::create_mock_host_instance;
    use super::*;
    use env_logger;
    use std::env;
    #[test]
    fn test_big_int_plus() {
        env_logger::try_init().unwrap_or_default();
        let test_wasm_file_path = env::var("TEST_WASM_FILE").expect("Test Wasm file not found");
        let (mut store, instance) = create_mock_host_instance(&test_wasm_file_path).unwrap();
        let f = instance.exports.get_function("testBigIntPlus").unwrap();
        log::info!("-- calling");
        let result = f.call(&mut store, &[]).unwrap();
        log::info!("-- result: {:?}", result);
        let ast_ptr = result[0].unwrap_i32();

        let memory = instance.exports.get_memory("memory").unwrap();
        let mut view = memory.view(&store);
        let data_size = view.data_size();
        let len = data_size - ast_ptr as u64;
        let mut buffer = vec![0; len as usize];
        view.read(ast_ptr as u64, &mut buffer).unwrap();
        let big_int_result: BigInt = BigInt::from_signed_bytes_le(&buffer).unwrap();
        log::info!("-- big_int_x: {}", big_int_x.to_string());
        assert_eq!(BigInt::from(19998), big_int_result)
    }
}
