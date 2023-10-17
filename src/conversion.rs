use wasmer::RuntimeError;
use wasmer::Type;
use wasmer::Value;

pub const CONVERSION_TYPE: ([Type; 1], [Type; 1]) = ([Type::I32], [Type::I32]);

pub fn big_int_to_hex(_: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    todo!()
}

pub fn big_int_to_string(_: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    todo!()
}

pub fn bytes_to_hex(_: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    todo!()
}
