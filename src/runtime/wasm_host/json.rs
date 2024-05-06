use super::Env;
use df_types::asc::base::asc_get;
use df_types::asc::base::asc_new;
use df_types::asc::base::AscPtr;
use df_types::asc::bignumber::AscBigInt;
use df_types::asc::native_types::string::AscString;
use df_types::bignumber::bigint::BigInt;
use df_types::wasmer::FunctionEnvMut;
use df_types::wasmer::RuntimeError;
use std::str::FromStr;

pub fn json_to_bigint(
    mut fenv: FunctionEnvMut<Env>,
    json_value_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let value: String = asc_get(&fenv, json_value_ptr, 0)?;
    let value = BigInt::from_str(&value)?;
    let asc_bigint = asc_new(&mut fenv, &value)?;
    Ok(asc_bigint)
}
