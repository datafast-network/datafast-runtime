use super::base::AscHeap;
use super::base::FromAscObj;
use super::base::ToAscObj;
use super::errors::AscError;
use super::native_types::Uint8Array;

use crate::bignumber::bigint::BigInt;

pub type AscBigInt = Uint8Array;

impl ToAscObj<AscBigInt> for BigInt {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscBigInt, AscError> {
        let bytes = self.to_signed_bytes_le();
        bytes.to_asc_obj(heap)
    }
}

impl FromAscObj<AscBigInt> for BigInt {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        array_buffer: AscBigInt,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let bytes = <Vec<u8>>::from_asc_obj(array_buffer, heap, depth)?;
        Ok(BigInt::from_signed_bytes_le(&bytes)?)
    }
}
