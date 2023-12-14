use bigdecimal;
use bigdecimal::FromPrimitive;
use serde;
use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Sub;
use std::str::FromStr;

use super::bigint::BigInt;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "bigdecimal::BigDecimal")]
pub struct BigDecimal(bigdecimal::BigDecimal);

impl From<bigdecimal::BigDecimal> for BigDecimal {
    fn from(big_decimal: bigdecimal::BigDecimal) -> Self {
        BigDecimal(big_decimal).normalized()
    }
}

impl BigDecimal {
    /// These are the limits of IEEE-754 decimal128, a format we may want to switch to. See
    /// https://en.wikipedia.org/wiki/Decimal128_floating-point_format.
    pub const MIN_EXP: i32 = -6143;
    pub const MAX_EXP: i32 = 6144;
    pub const MAX_SIGNFICANT_DIGITS: i32 = 34;

    pub fn new(digits: BigInt, exp: i64) -> Self {
        // bigdecimal uses `scale` as the opposite of the power of ten, so negate `exp`.
        Self::from(bigdecimal::BigDecimal::new(digits.inner(), -exp))
    }

    pub fn parse_bytes(bytes: &[u8]) -> Option<Self> {
        bigdecimal::BigDecimal::parse_bytes(bytes, 10).map(Self)
    }

    pub fn zero() -> BigDecimal {
        use bigdecimal::Zero;

        BigDecimal(bigdecimal::BigDecimal::zero())
    }

    pub fn as_bigint_and_exponent(&self) -> (num_bigint::BigInt, i64) {
        self.0.as_bigint_and_exponent()
    }

    pub fn digits(&self) -> u64 {
        self.0.digits()
    }

    // Copy-pasted from `bigdecimal::BigDecimal::normalize`. We can use the upstream version once it
    // is included in a released version supported by Diesel.
    #[must_use]
    pub fn normalized(&self) -> BigDecimal {
        if self == &BigDecimal::zero() {
            return BigDecimal::zero();
        }

        // Round to the maximum significant digits.
        let big_decimal = self.0.with_prec(Self::MAX_SIGNFICANT_DIGITS as u64);

        let (bigint, exp) = big_decimal.as_bigint_and_exponent();
        let (sign, mut digits) = bigint.to_radix_be(10);
        let trailing_count = digits.iter().rev().take_while(|i| **i == 0).count();
        digits.truncate(digits.len() - trailing_count);
        let int_val = num_bigint::BigInt::from_radix_be(sign, &digits, 10).unwrap();
        let scale = exp - trailing_count as i64;

        BigDecimal(bigdecimal::BigDecimal::new(int_val, scale))
    }
}

impl Display for BigDecimal {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl fmt::Debug for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BigDecimal({})", self.0)
    }
}

impl FromStr for BigDecimal {
    type Err = <bigdecimal::BigDecimal as FromStr>::Err;

    fn from_str(s: &str) -> Result<BigDecimal, Self::Err> {
        Ok(Self::from(bigdecimal::BigDecimal::from_str(s)?))
    }
}

impl From<i32> for BigDecimal {
    fn from(n: i32) -> Self {
        Self::from(bigdecimal::BigDecimal::from(n))
    }
}

impl From<i64> for BigDecimal {
    fn from(n: i64) -> Self {
        Self::from(bigdecimal::BigDecimal::from(n))
    }
}

impl From<u64> for BigDecimal {
    fn from(n: u64) -> Self {
        Self::from(bigdecimal::BigDecimal::from(n))
    }
}

impl From<f64> for BigDecimal {
    fn from(n: f64) -> Self {
        // FIX: try to unwrap here, but it should not be so
        Self::from(bigdecimal::BigDecimal::from_f64(n).unwrap())
    }
}

impl Add for BigDecimal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::from(self.0.add(other.0))
    }
}

impl Sub for BigDecimal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::from(self.0.sub(other.0))
    }
}

impl Mul for BigDecimal {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::from(self.0.mul(other.0))
    }
}

impl Div for BigDecimal {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self::from(self.0.div(other.0))
    }
}
