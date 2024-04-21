use super::Env;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::bignumber::AscBigInt;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::bignumber::bigint::BigInt;
use std::str::FromStr;
use serde_json::Value;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;
use crate::runtime::asc::native_types::{Uint8Array};
use crate::runtime::asc::native_types::json::JsonValueKind;
use crate::runtime::asc::native_types::r#enum::AscEnum;

pub fn json_to_bigint(
    mut fenv: FunctionEnvMut<Env>,
    json_value_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscBigInt>, RuntimeError> {
    let value: String = asc_get(&fenv, json_value_ptr, 0)?;
    let value = BigInt::from_str(&value)?;
    let asc_bigint = asc_new(&mut fenv, &value)?;
    Ok(asc_bigint)
}

pub fn json_from_bytes(
    mut fenv: FunctionEnvMut<Env>,
    bytes_ptr: AscPtr<Uint8Array>,
) -> Result<AscPtr<AscEnum<JsonValueKind>>, RuntimeError> {
    let bytes = asc_get::<Vec<u8>, _, _>(&fenv, bytes_ptr, 0)?;
    let bytes = serde_json::from_slice::<Value>(&bytes).map_err(|e| RuntimeError::new(e.to_string()))?;
    asc_new(&mut fenv, &bytes).map_err(|e| RuntimeError::new(e.to_string()))
}