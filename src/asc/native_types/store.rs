use crate::asc::base::AscValue;
use crate::db_worker::abstract_types::Value;
use crate::impl_asc_type_enum;
use semver::Version;

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum StoreValueKind {
    String,
    Int,
    BigDecimal,
    Bool,
    Array,
    Null,
    Bytes,
    BigInt,
    Int8,
}

impl_asc_type_enum!(
    StoreValueKind;
    String => 0,
    Int => 1,
    BigDecimal => 2,
    Bool => 3,
    Array => 4,
    Null => 5,
    Bytes => 6,
    BigInt => 7,
    Int8 => 8
);

impl StoreValueKind {
    pub fn get_kind(value: &Value) -> StoreValueKind {
        match value {
            Value::String(_) => StoreValueKind::String,
            Value::Int(_) => StoreValueKind::Int,
            Value::Int8(_) => StoreValueKind::Int8,
            Value::BigDecimal(_) => StoreValueKind::BigDecimal,
            Value::Bool(_) => StoreValueKind::Bool,
            Value::List(_) => StoreValueKind::Array,
            Value::Null => StoreValueKind::Null,
            Value::Bytes(_) => StoreValueKind::Bytes,
            Value::BigInt(_) => StoreValueKind::BigInt,
        }
    }
}

impl Default for StoreValueKind {
    fn default() -> Self {
        StoreValueKind::Null
    }
}

impl AscValue for StoreValueKind {}
