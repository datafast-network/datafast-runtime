use df_types::asc::base::asc_new;
use df_types::semver::Version;

use df_types::asc::native_types::Uint8Array;
use df_types::asc::base::asc_get;
use df_types::asc::base::AscHeap;
use df_types::asc::base::AscIndexId;
use df_types::asc::base::AscPtr;
use df_types::asc::base::FromAscObj;
use df_types::asc::base::IndexForAscTypeId;
use df_types::asc::base::ToAscObj;
use df_types::errors::AscError;

use crate::impl_asc_type_struct;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;

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

#[repr(C)]
pub struct AscBigDecimal {
    pub digits: AscPtr<AscBigInt>,
    // Decimal exponent. This is the opposite of `scale` in rust BigDecimal.
    pub exp: AscPtr<AscBigInt>,
}

impl_asc_type_struct!(
    AscBigDecimal;
    digits => AscPtr<AscBigInt>,
    exp => AscPtr<AscBigInt>
);

impl AscIndexId for AscBigDecimal {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::BigDecimal;
}

impl ToAscObj<AscBigDecimal> for BigDecimal {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscBigDecimal, AscError> {
        // From the docs: "Note that a positive exponent indicates a negative power of 10",
        // so "exponent" is the opposite of what you'd expect.
        let (digits, negative_exp) = self.as_bigint_and_exponent();
        Ok(AscBigDecimal {
            exp: asc_new(heap, &BigInt::from(-negative_exp))?,
            digits: asc_new(heap, &BigInt::new(digits)?)?,
        })
    }
}

impl FromAscObj<AscBigDecimal> for BigDecimal {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        big_decimal: AscBigDecimal,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let digits: BigInt = asc_get(heap, big_decimal.digits, depth)?;
        let exp: BigInt = asc_get(heap, big_decimal.exp, depth)?;

        let bytes = exp.to_signed_bytes_le();
        let mut byte_array = if exp >= 0.into() { [0; 8] } else { [255; 8] };
        byte_array[..bytes.len()].copy_from_slice(&bytes);
        let big_decimal = BigDecimal::new(digits, i64::from_le_bytes(byte_array));

        // Validate the exponent.
        let exp = -big_decimal.as_bigint_and_exponent().1;
        let min_exp: i64 = BigDecimal::MIN_EXP.into();
        let max_exp: i64 = BigDecimal::MAX_EXP.into();
        if exp < min_exp || max_exp < exp {
            Err(AscError::Plain(format!(
                "big decimal exponent `{}` is outside the `{}` to `{}` range",
                exp, min_exp, max_exp
            )))
        } else {
            Ok(big_decimal)
        }
    }
}
