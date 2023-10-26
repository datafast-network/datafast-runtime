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
    let x: BigInt = asc_get(&fenv, bigint_x_ptr, 0)?;
    let y: BigInt = asc_get(&fenv, bigint_y_ptr, 0)?;
    let result = x + y;
    let asc_pt = asc_new(&mut fenv, &result)?;
    Ok(asc_pt)
}

#[cfg(test)]
mod tests {
    use super::super::test::*;
    use crate::asc::base::asc_get;
    use crate::asc::base::AscPtr;
    use crate::asc::bignumber::AscBigInt;
    use crate::bignumber::bigint::BigInt;
    use crate::host_fn_test;

    host_fn_test!(test_big_int_plus, host, ptr {
        let asc_ptr = AscPtr::<AscBigInt>::new(ptr as u32);
        let bigint_result: BigInt = asc_get(&host, asc_ptr, 0).unwrap();
        assert_eq!(bigint_result.to_string(), "3000");
    });
}
