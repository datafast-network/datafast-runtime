use crate::runtime::asc::native_types::store::Value;
use scylla::_macro_internal::CqlValue;
#[macro_export]
macro_rules! schema {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        use $crate::components::manifest_loader::schema_lookup::FieldKind;
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), FieldKind{
            kind: $v,
            relation: None,
            list_inner_kind: None,
        }),)*]))
    }};
}

#[macro_export]
macro_rules! entity {
    ($($k:ident => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$((stringify!($k).to_string(), $v),)*]))
    }};
}

impl From<Value> for CqlValue {
    fn from(value: Value) -> Self {
        match value {
            Value::String(str) => CqlValue::Text(str),
            Value::Int(int) => CqlValue::Int(int),
            Value::Int8(int8) => CqlValue::BigInt(int8),
            Value::BigDecimal(decimal) => CqlValue::Text(decimal.to_string()),
            Value::Bool(bool) => CqlValue::Boolean(bool),
            Value::List(list) => CqlValue::List(list.into_iter().map(CqlValue::from).collect()),
            Value::Bytes(bytes) => CqlValue::Blob(bytes.as_slice().to_vec()),
            Value::BigInt(n) => CqlValue::Text(n.to_string()),
            Value::Null => CqlValue::Empty,
        }
    }
}
