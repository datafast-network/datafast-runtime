use crate::impl_asc_type_enum;
use crate::runtime::asc::base::AscValue;
use semver::Version;

use super::r#enum::AscEnum;
use super::string::AscString;
use super::typed_map::AscTypedMap;

#[repr(u32)]
#[derive(Copy, Clone)]
#[derive(Default)]
pub enum JsonValueKind {
    #[default]
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl_asc_type_enum!(
    JsonValueKind;
    Null => 0,
    Bool => 1,
    Number => 2,
    String => 3,
    Array => 4,
    Object => 5
);



impl AscValue for JsonValueKind {}

impl JsonValueKind {
    pub fn get_kind(token: &serde_json::Value) -> Self {
        use serde_json::Value;

        match token {
            Value::Null => JsonValueKind::Null,
            Value::Bool(_) => JsonValueKind::Bool,
            Value::Number(_) => JsonValueKind::Number,
            Value::String(_) => JsonValueKind::String,
            Value::Array(_) => JsonValueKind::Array,
            Value::Object(_) => JsonValueKind::Object,
        }
    }
}

pub type AscJson = AscTypedMap<AscString, AscEnum<JsonValueKind>>;
