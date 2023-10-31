use std::env;

use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::native_types::string::AscString;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn store_set(fenv: FunctionEnvMut<Env>, _: i32, _: i32) -> Result<(), RuntimeError> {
    Ok(())
}

pub fn store_get(fenv: FunctionEnvMut<Env>, _: i32, _: i32) -> Result<(), RuntimeError> {
    Ok(())
}
