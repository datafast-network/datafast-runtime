use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscPtr;
use crate::asc::bignumber::AscBigInt;
use crate::asc::native_types::string::AscString;
use crate::bignumber::bigint::BigInt;
use std::str::FromStr;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn json_to_bigint(
    mut fenv: FunctionEnvMut<Env>,
    json_value_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let value: String = asc_get(&fenv, json_value_ptr, 0)?;
    let value = BigInt::from_str(&value)?;
    let asc_bigint = asc_new(&mut fenv, &value)?;
    Ok(asc_bigint)
}
