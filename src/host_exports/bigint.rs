#![allow(dead_code)]
use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigDecimal;
use crate::asc::bignumber::AscBigInt;
use crate::asc::native_types::string::AscString;
use crate::bignumber::bigdecimal::BigDecimal;
use crate::bignumber::bigint::BigInt;
use std::str::FromStr;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn big_int_plus(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    let result = x + y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_minus(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    let result = x - y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_times(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    let result = x * y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_divided_by(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    if y == BigInt::from(0) {
        return Err(RuntimeError::new("Divide by zero error!"));
    }
    let result = x / y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_bit_or(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    if y == 0.into() {
        return Err(RuntimeError::new("Divide by zero error!"));
    }
    let result = x | y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_bit_and(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    if y == 0.into() {
        return Err(RuntimeError::new("Divide by zero error!"));
    }
    let result = x & y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_divided_by_decimal(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let x: BigDecimal = BigDecimal::new(asc_get(&fenv, bigint_x_ptr, 0)?, 0);
    let y: BigDecimal = asc_get(&fenv, bigint_y_ptr, 0)?;
    if y == 0.into() {
        return Err(RuntimeError::new("Divide by zero error!"));
    }
    let result = x / y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_mod(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    bigint_y_ptr: AscPtr<AscBigInt>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    if y == 0.into() {
        return Err(RuntimeError::new("Divide by zero error!"));
    }
    let result = x % y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_pow(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    exp: i32,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let result = x.pow(exp as u32)?;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_left_shift(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    exp: i32,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let bits = u8::try_from(exp)
        .map_err(|_| RuntimeError::new("Exponent must be a positive integer less than 256"))?;
    let result = x << bits;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_right_shift(
    mut fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: AscPtr<AscBigInt>,
    exp: i32,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let bits = u8::try_from(exp)
        .map_err(|_| RuntimeError::new("Exponent must be a positive integer less than 256"))?;
    let result = x >> bits;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_int_from_string(
    mut fenv: FunctionEnvMut<Env>,
    string_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let x: String = asc_get(&fenv, string_ptr, 0)?;
    let result = BigInt::from_str(&x)?;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

#[cfg(test)]
mod tests {
    use super::super::test::create_mock_host_instance;
    use crate::asc::base::asc_get;
    use crate::asc::base::AscPtr;
    use crate::asc::bignumber::AscBigInt;
    use crate::bignumber::bigint::BigInt;
    use std::env;

    #[test]
    fn test_big_int_plus() {
        env_logger::try_init().unwrap_or_default();
        let test_wasm_file_path = env::var("TEST_WASM_FILE").expect("Test Wasm file not found");
        log::info!("Test Wasm path: {test_wasm_file_path}");
        let mut host = create_mock_host_instance(&test_wasm_file_path).unwrap();
        let f = host
            .instance
            .exports
            .get_function("testBigIntPlus")
            .unwrap();
        let result = f.call(&mut host.store, &[]).unwrap();
        let ptr = result.first().unwrap().unwrap_i32();
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr as u32);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "3000");
    }
}
