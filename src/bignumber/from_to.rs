use crate::bignumber::bigint::BigInt;
use std::str::FromStr;
use web3::types as w3;

impl From<BigInt> for w3::U64 {
    fn from(value: BigInt) -> Self {
        Self::from_dec_str(&value.to_string()).unwrap()
    }
}

impl From<BigInt> for w3::U256 {
    fn from(big_int: BigInt) -> Self {
        Self::from_str(&big_int.to_string()).unwrap()
    }
}

impl From<BigInt> for w3::U128 {
    fn from(big_int: BigInt) -> Self {
        Self::from_str(&big_int.to_string()).unwrap()
    }
}
