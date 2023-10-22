use wasmer::RuntimeError;
use wasmer::Type;
use wasmer::Value;

pub const STORE_SET_TYPE: ([Type; 3], [Type; 0]) = ([Type::I32, Type::I32, Type::I32], []);

pub fn store_set(_: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    todo!()
}

pub const STORE_GET_TYPE: ([Type; 2], [Type; 1]) = ([Type::I32, Type::I32], [Type::I32]);

pub fn store_get(_: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    todo!()
}