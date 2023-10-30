use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigDecimal;
use crate::asc::native_types::string::AscString;
use crate::bignumber::bigdecimal::BigDecimal;
use crate::wasm_host::Env;
use std::str::FromStr;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn big_decimal_plus(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
    big_decimal_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let x: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let y: BigDecimal = asc_get(&fenv, big_decimal_y_ptr, 0)?;
    let result = x + y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_decimal_minus(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
    big_decimal_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let x: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let y: BigDecimal = asc_get(&fenv, big_decimal_y_ptr, 0)?;
    let result = x - y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_decimal_times(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
    big_decimal_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let x: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let y: BigDecimal = asc_get(&fenv, big_decimal_y_ptr, 0)?;

    if y == BigDecimal::from(0) {
        return Err(RuntimeError::new("Divide by zero"));
    }

    let result = x * y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_decimal_divided_by(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
    big_decimal_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let x: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let y: BigDecimal = asc_get(&fenv, big_decimal_y_ptr, 0)?;

    if y == BigDecimal::from(0) {
        return Err(RuntimeError::new("Divide by zero"));
    }

    let result = x / y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_decimal_from_string(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscBigDecimal>, RuntimeError> {
    let string: String = asc_get(&fenv, big_decimal_x_ptr, 0)?;

    let result = BigDecimal::from_str(&string)
        .map_err(|e| RuntimeError::new(format!("Error parsing BigDecimal from string: {}", e)))?;
    let asc_pt = asc_new(&mut fenv, &result)?;

    Ok(asc_pt)
}

pub fn big_decimal_to_string(
    mut fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let big_decimal: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let result = big_decimal.to_string();
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

pub fn big_decimal_equals(
    fenv: FunctionEnvMut<Env>,
    big_decimal_x_ptr: AscPtr<AscBigDecimal>,
    big_decimal_y_ptr: AscPtr<AscBigDecimal>,
) -> Result<i32, RuntimeError> {
    let x: BigDecimal = asc_get(&fenv, big_decimal_x_ptr, 0)?;
    let y: BigDecimal = asc_get(&fenv, big_decimal_y_ptr, 0)?;
    let result = x == y;
    Ok(result as i32)
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use crate::asc::base::asc_get;
    use crate::asc::base::AscPtr;
    use crate::asc::bignumber::AscBigDecimal;
    use crate::asc::native_types::string::AscString;
    use crate::bignumber::bigdecimal::BigDecimal;
    use crate::host_fn_test;

    host_fn_test!(test_big_decimal_plus, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "3000");
    });

    host_fn_test!(test_big_decimal_minus, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "-1000");
    });

    host_fn_test!(test_big_decimal_times, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "2000000");
    });

    host_fn_test!(test_big_decimal_divided_by, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "0.5");
    });

    host_fn_test!(test_big_decimal_from_string, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "1000");
    });

    host_fn_test!(test_big_decimal_to_string, host, ptr {
        let asc_ptr = AscPtr::<AscString>::new(ptr);
        let bigint_result: String = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result, "1000");
    });

    host_fn_test!(test_big_decimal_equals, host, ptr {
        assert_eq!(ptr, 1);
    });
}
