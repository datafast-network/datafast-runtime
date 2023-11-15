use crate::errors::BigIntOutOfRangeError;
use crate::errors::BigNumberErr;
use num_bigint;
use serde::Deserialize;
use serde::Serialize;
use web3::types as web3;

pub use num_bigint::Sign as BigIntSign;

use num_traits::Num;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::f32::consts::LOG2_10;
use std::fmt;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;
use std::ops::Add;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Rem;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Sub;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigInt(num_bigint::BigInt);

impl Display for BigInt {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        self.0.fmt(f)
    }
}

impl fmt::Debug for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BigInt({})", self)
    }
}

impl BigInt {
    // Postgres `numeric` has a limit documented here [https://www.postgresql.org/docs/current/datatype-numeric.htm]:
    // "Up to 131072 digits before the decimal point; up to 16383 digits after the decimal point"
    // So based on this we adopt a limit of 131072 decimal digits for big int, converted here to bits.
    pub const MAX_BITS: u32 = (131072.0 * LOG2_10) as u32 + 1; // 435_412

    pub fn new(inner: num_bigint::BigInt) -> Result<Self, BigNumberErr> {
        // `inner.bits()` won't include the sign bit, so we add 1 to account for it.
        let bits = inner.bits() as usize + 1;
        if bits > Self::MAX_BITS as usize {
            return Err(BigNumberErr::NumberTooBig);
        }
        Ok(Self(inner))
    }

    /// Creates a BigInt without checking the digit limit.
    pub fn unchecked_new(inner: num_bigint::BigInt) -> Self {
        Self(inner)
    }

    pub fn sign(&self) -> num_bigint::Sign {
        self.0.sign()
    }

    pub fn to_bytes_le(&self) -> (BigIntSign, Vec<u8>) {
        self.0.to_bytes_le()
    }

    pub fn to_bytes_be(&self) -> (BigIntSign, Vec<u8>) {
        self.0.to_bytes_be()
    }

    pub fn to_signed_bytes_le(&self) -> Vec<u8> {
        self.0.to_signed_bytes_le()
    }

    pub fn bits(&self) -> usize {
        self.0.bits() as usize
    }

    pub fn inner(self) -> num_bigint::BigInt {
        self.0
    }
}

impl<'a> TryFrom<&'a BigInt> for u64 {
    type Error = BigIntOutOfRangeError;
    fn try_from(value: &'a BigInt) -> Result<u64, BigIntOutOfRangeError> {
        let (sign, bytes) = value.to_bytes_le();

        if sign == num_bigint::Sign::Minus {
            return Err(BigIntOutOfRangeError::Negative);
        }

        if bytes.len() > 8 {
            return Err(BigIntOutOfRangeError::Overflow);
        }

        // Replace this with u64::from_le_bytes when stabilized
        let mut n = 0u64;
        let mut shift_dist = 0;
        for b in bytes {
            n |= (b as u64) << shift_dist;
            shift_dist += 8;
        }
        Ok(n)
    }
}

impl TryFrom<BigInt> for u64 {
    type Error = BigIntOutOfRangeError;
    fn try_from(value: BigInt) -> Result<u64, BigIntOutOfRangeError> {
        (&value).try_into()
    }
}

impl BigInt {
    pub fn from_unsigned_bytes_le(bytes: &[u8]) -> Result<Self, BigNumberErr> {
        BigInt::new(num_bigint::BigInt::from_bytes_le(
            num_bigint::Sign::Plus,
            bytes,
        ))
    }

    pub fn from_signed_bytes_le(bytes: &[u8]) -> Result<Self, BigNumberErr> {
        BigInt::new(num_bigint::BigInt::from_signed_bytes_le(bytes))
    }

    pub fn from_signed_bytes_be(bytes: &[u8]) -> Result<Self, BigNumberErr> {
        BigInt::new(num_bigint::BigInt::from_signed_bytes_be(bytes))
    }

    pub fn from_unsigned_u128(n: web3::U128) -> Self {
        let mut bytes: [u8; 16] = [0; 16];
        n.to_little_endian(&mut bytes);
        // Unwrap: 128 bits is much less than BigInt::MAX_BITS
        BigInt::from_unsigned_bytes_le(&bytes).unwrap()
    }

    pub fn from_unsigned_u256(n: &web3::U256) -> Self {
        let mut bytes: [u8; 32] = [0; 32];
        n.to_little_endian(&mut bytes);
        // Unwrap: 256 bits is much less than BigInt::MAX_BITS
        BigInt::from_unsigned_bytes_le(&bytes).unwrap()
    }

    pub fn from_signed_u256(n: &web3::U256) -> Self {
        let mut bytes: [u8; 32] = [0; 32];
        n.to_little_endian(&mut bytes);
        BigInt::from_signed_bytes_le(&bytes).unwrap()
    }

    pub fn to_signed_u256(&self) -> web3::U256 {
        let bytes = self.to_signed_bytes_le();
        if self < &BigInt::from(0) {
            assert!(
                bytes.len() <= 32,
                "BigInt value does not fit into signed U256"
            );
            let mut i_bytes: [u8; 32] = [255; 32];
            i_bytes[..bytes.len()].copy_from_slice(&bytes);
            web3::U256::from_little_endian(&i_bytes)
        } else {
            web3::U256::from_little_endian(&bytes)
        }
    }

    pub fn to_unsigned_u256(&self) -> web3::U256 {
        let (sign, bytes) = self.to_bytes_le();
        assert!(
            sign == BigIntSign::NoSign || sign == BigIntSign::Plus,
            "negative value encountered for U256: {}",
            self
        );
        web3::U256::from_little_endian(&bytes)
    }

    pub fn to_unsigned_u64(&self) -> web3::U64 {
        let (sign, bytes) = self.to_bytes_le();
        assert!(
            sign == BigIntSign::NoSign || sign == BigIntSign::Plus,
            "negative value encountered for U256: {}",
            self
        );
        web3::U64::from_little_endian(&bytes)
    }

    /// Exponential a `BigInt` to a `u32` power.
    pub fn pow(self, exponent: u32) -> Result<BigInt, BigNumberErr> {
        use num_traits::pow::Pow;

        BigInt::new(self.inner().pow(exponent))
    }

    pub fn from_hex(hex: String) -> Result<BigInt, BigNumberErr> {
        let big_int = num_bigint::BigInt::from_str_radix(&hex, 16)?;
        BigInt::new(big_int)
    }
}

impl From<i32> for BigInt {
    fn from(i: i32) -> BigInt {
        BigInt::unchecked_new(i.into())
    }
}

impl From<u64> for BigInt {
    fn from(i: u64) -> BigInt {
        BigInt::unchecked_new(i.into())
    }
}

impl From<i64> for BigInt {
    fn from(i: i64) -> BigInt {
        BigInt::unchecked_new(i.into())
    }
}

impl From<web3::U64> for BigInt {
    /// This implementation assumes that U64 represents an unsigned U64,
    /// and not a signed U64 (aka int64 in Solidity). Right now, this is
    /// all we need (for block numbers). If it ever becomes necessary to
    /// handle signed U64s, we should add the same
    /// `{to,from}_{signed,unsigned}_u64` methods that we have for U64.
    fn from(n: web3::U64) -> BigInt {
        BigInt::from(n.as_u64())
    }
}

impl FromStr for BigInt {
    type Err = BigNumberErr;

    fn from_str(s: &str) -> Result<BigInt, Self::Err> {
        num_bigint::BigInt::from_str(s)
            .map_err(|_| BigNumberErr::Parser)
            .and_then(BigInt::new)
    }
}

impl Serialize for BigInt {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BigInt {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        let decimal_string = <String>::deserialize(deserializer)?;
        BigInt::from_str(&decimal_string).map_err(D::Error::custom)
    }
}

impl Add for BigInt {
    type Output = BigInt;

    fn add(self, other: BigInt) -> BigInt {
        BigInt::unchecked_new(self.inner().add(other.inner()))
    }
}

impl Sub for BigInt {
    type Output = BigInt;

    fn sub(self, other: BigInt) -> BigInt {
        BigInt::unchecked_new(self.inner().sub(other.inner()))
    }
}

impl Mul for BigInt {
    type Output = BigInt;

    fn mul(self, other: BigInt) -> BigInt {
        BigInt::unchecked_new(self.inner().mul(other.inner()))
    }
}

impl Div for BigInt {
    type Output = BigInt;

    fn div(self, other: BigInt) -> BigInt {
        if other == BigInt::from(0) {
            panic!("Cannot divide by zero-valued `BigInt`!")
        }

        BigInt::unchecked_new(self.inner().div(other.inner()))
    }
}

impl Rem for BigInt {
    type Output = BigInt;

    fn rem(self, other: BigInt) -> BigInt {
        BigInt::unchecked_new(self.inner().rem(other.inner()))
    }
}

impl BitOr for BigInt {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        BigInt::unchecked_new(self.inner().bitor(other.inner()))
    }
}

impl BitAnd for BigInt {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        BigInt::unchecked_new(self.inner().bitand(other.inner()))
    }
}

impl Shl<u8> for BigInt {
    type Output = Self;

    fn shl(self, bits: u8) -> Self {
        BigInt::unchecked_new(self.inner().shl(bits))
    }
}

impl Shr<u8> for BigInt {
    type Output = Self;

    fn shr(self, bits: u8) -> Self {
        BigInt::unchecked_new(self.inner().shr(bits))
    }
}

impl From<BigInt> for web3::U64 {
    fn from(big_int: BigInt) -> Self {
        Self::from_little_endian(&big_int.to_signed_bytes_le())
    }
}

impl From<BigInt> for web3::U256 {
    fn from(big_int: BigInt) -> Self {
        Self::from_little_endian(&big_int.to_signed_bytes_le())
    }
}

impl From<BigInt> for web3::U128 {
    fn from(big_int: BigInt) -> Self {
        Self::from_little_endian(&big_int.to_signed_bytes_le())
    }
}
