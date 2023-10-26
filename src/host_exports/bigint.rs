use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigInt;
use crate::bignumber::bigint::BigInt;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn big_int_plus(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let bigint_x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let bigint_y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;

    log::info!("bigint_x: {bigint_x}");
    log::info!("bigint_y: {bigint_y}");

    let result = bigint_x + bigint_y;
    let asc_pt = asc_new(&mut fenv, &result)?;

    log::info!("result: {result}");
    log::info!("asc_pt: {}", asc_pt.wasm_ptr());
    Ok(asc_pt)
}

#[cfg(test)]
mod tests {
    use super::super::test::create_mock_host_instance;
    use std::env;

    #[test]
    fn test_big_int_plus() {
        env_logger::try_init().unwrap_or_default();
        let test_wasm_file_path = env::var("TEST_WASM_FILE").expect("Test Wasm file not found");
        log::info!("Test Wasm path: {test_wasm_file_path}");
        let (mut store, instance) = create_mock_host_instance(&test_wasm_file_path).unwrap();
        let f = instance.exports.get_function("testBigIntPlus").unwrap();
        log::info!("-- calling");
        let result = f.call(&mut store, &[]).unwrap();
        assert!(result.first().is_some());
        let pt = result.first().unwrap().unwrap_i32();
        log::info!("-- result: {pt}");
    }
}
