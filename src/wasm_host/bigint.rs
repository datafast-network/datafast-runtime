use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigDecimal;
use crate::asc::bignumber::AscBigInt;
use crate::asc::native_types::string::AscString;
use crate::bignumber::bigdecimal::BigDecimal;
use crate::bignumber::bigint::BigInt;
use std::ops::Rem;
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
    // NOTE: 20 %-9 = 2 => Đéo hiểu tại sao = 2
    let result = x.rem(y);
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
    use super::super::test::*;
    use crate::asc::base::asc_get;
    use crate::asc::base::AscPtr;
    use crate::asc::bignumber::AscBigDecimal;
    use crate::asc::bignumber::AscBigInt;
    use crate::bignumber::bigdecimal::BigDecimal;
    use crate::bignumber::bigint::BigInt;
    use crate::host_fn_test;

    host_fn_test!("test", test_big_int_plus, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "3000");
    });

    host_fn_test!("test", test_big_int_minus, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "-1000");
    });

    host_fn_test!("test", test_big_int_times, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "2000000");
    });

    host_fn_test!("test", test_big_int_divided_by, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "6");
    });

    host_fn_test!("test", test_big_int_pow, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "100000000000000000001");
    });

    host_fn_test!("test", test_big_int_mod, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "9");
    });

    host_fn_test!("test", test_big_int_bit_or, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "2040");
    });

    host_fn_test!("test", test_big_int_bit_and, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "960");
    });

    host_fn_test!("test", test_big_int_divided_by_decimal, host, ptr {
        let asc_ptr = AscPtr::<AscBigDecimal>::new(ptr);
        let bigint_result: BigDecimal = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "0.5");
    });

    host_fn_test!("test", test_big_int_left_shift, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "1024000");
    });

    host_fn_test!("test", test_big_int_right_shift, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "0");
    });
}
