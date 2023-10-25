use super::Env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn big_int_plus(
    fenv: FunctionEnvMut<Env>,
    bigint_x_ptr: i32,
    bigint_y_ptr: i32,
) -> Result<i32, RuntimeError> {
    todo!()
}
