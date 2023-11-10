use crate::impl_asc_type_enum;
use crate::runtime::asc::base::AscValue;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;
use std::str::FromStr;
use web3::types::Address;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes(Box<[u8]>);

impl Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes(0x{})", hex::encode(&self.0))
    }
}

impl Bytes {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Bytes {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl FromStr for Bytes {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Bytes, Self::Err> {
        hex::decode(s.trim_start_matches("0x")).map(|x| Bytes(x.into()))
    }
}

impl<'a> From<&'a [u8]> for Bytes {
    fn from(array: &[u8]) -> Self {
        Bytes(array.into())
    }
}

impl From<Address> for Bytes {
    fn from(address: Address) -> Bytes {
        Bytes::from(address.as_ref())
    }
}

impl From<web3::types::Bytes> for Bytes {
    fn from(bytes: web3::types::Bytes) -> Bytes {
        Bytes::from(bytes.0.as_slice())
    }
}

impl Serialize for Bytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        let hex_string = <String>::deserialize(deserializer)?;
        Bytes::from_str(&hex_string).map_err(D::Error::custom)
    }
}

impl<const N: usize> From<[u8; N]> for Bytes {
    fn from(array: [u8; N]) -> Bytes {
        Bytes(array.into())
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(vec: Vec<u8>) -> Self {
        Bytes(vec.into())
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Value {
    String(String),
    Int(i32),
    Int8(i64),
    BigDecimal(BigDecimal),
    Bool(bool),
    List(Vec<Value>),
    Null,
    Bytes(Bytes),
    BigInt(BigInt),
}
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
#[derive(Default)]
pub enum StoreValueKind {
    String,
    Int,
    BigDecimal,
    Bool,
    Array,
    #[default]
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



impl AscValue for StoreValueKind {}
