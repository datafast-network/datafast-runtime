use crate::bignumber::bigint::BigInt;
use std::str::FromStr;
use web3::types::U256;
use web3::types::U64;

impl From<BigInt> for U64 {
    fn from(big_int: BigInt) -> Self {
        U64::from_dec_str(&big_int.to_string()).unwrap()
    }
}

impl From<BigInt> for U256 {
    fn from(big_int: BigInt) -> Self {
        U256::from_str(&big_int.to_string()).unwrap()
    }
}
