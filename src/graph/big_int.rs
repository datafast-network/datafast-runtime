use num_bigint;
use std::{
    f32::consts::LOG2_10,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigInt(num_bigint::BigInt);

impl Display for BigInt {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl BigInt {
    // Postgres `numeric` has a limit documented here [https://www.postgresql.org/docs/current/datatype-numeric.htm]:
    // "Up to 131072 digits before the decimal point; up to 16383 digits after the decimal point"
    // So based on this we adopt a limit of 131072 decimal digits for big int, converted here to bits.
    pub const MAX_BITS: u32 = (131072.0 * LOG2_10) as u32 + 1; // 435_412

    pub fn new(inner: num_bigint::BigInt) -> Result<Self, anyhow::Error> {
        // `inner.bits()` won't include the sign bit, so we add 1 to account for it.
        let bits = inner.bits() + 1;
        if bits > Self::MAX_BITS as usize {
            anyhow::bail!(
                    "BigInt is too big, total bits {} (max {})",
                    bits,
                    Self::MAX_BITS
                );
        }
        Ok(Self(inner))
    }

    /// Creates a BigInt without checking the digit limit.
    pub(super) fn unchecked_new(inner: num_bigint::BigInt) -> Self {
        Self(inner)
    }

    pub fn sign(&self) -> num_bigint::Sign {
        self.0.sign()
    }

    pub fn to_bytes_le(&self) -> (num_bigint::Sign, Vec<u8>) {
        self.0.to_bytes_le()
    }

    pub fn to_bytes_be(&self) -> (num_bigint::Sign, Vec<u8>) {
        self.0.to_bytes_be()
    }

    pub fn to_signed_bytes_le(&self) -> Vec<u8> {
        self.0.to_signed_bytes_le()
    }

    pub fn bits(&self) -> usize {
        self.0.bits()
    }

    pub(super) fn inner(self) -> num_bigint::BigInt {
        self.0
    }
}
