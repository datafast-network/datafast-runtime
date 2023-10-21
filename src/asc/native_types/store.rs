use crate::asc::AscValue;
use crate::impl_asc_type_enum;

#[repr(u32)]
#[derive(Copy, Clone)]
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

// TODO: determine store data type and impl this
// impl StoreValueKind {
//     pub(crate) fn get_kind(value: &store::Value) -> StoreValueKind {
//         use self::store::Value;

//         match value {
//             Value::String(_) => StoreValueKind::String,
//             Value::Int(_) => StoreValueKind::Int,
//             Value::Int8(_) => StoreValueKind::Int8,
//             Value::BigDecimal(_) => StoreValueKind::BigDecimal,
//             Value::Bool(_) => StoreValueKind::Bool,
//             Value::List(_) => StoreValueKind::Array,
//             Value::Null => StoreValueKind::Null,
//             Value::Bytes(_) => StoreValueKind::Bytes,
//             Value::BigInt(_) => StoreValueKind::BigInt,
//         }
//     }
// }

impl Default for StoreValueKind {
    fn default() -> Self {
        StoreValueKind::Null
    }
}

impl AscValue for StoreValueKind {}
