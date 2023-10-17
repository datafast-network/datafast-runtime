use wasmer::RuntimeError;
use wasmer::Type;
use wasmer::Value;

pub const ABORT_TYPE: ([Type; 4], [Type; 0]) = ([Type::I32, Type::I32, Type::I32, Type::I32], []);

pub fn abort(message: &[Value]) -> Result<Vec<Value>, RuntimeError> {
    println!(
        "-------- abort-message: {:?}",
        message.iter().map(|v| v.to_string()).collect::<Vec<_>>()
    );
    Ok(vec![])
}
