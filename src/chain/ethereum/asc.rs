use web3::types as web3;

use crate::asc::base::asc_get;
use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscValue;
use crate::asc::base::FromAscObj;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::asc::native_types::array::Array;
use crate::asc::native_types::r#enum::AscEnum;
use crate::asc::native_types::r#enum::AscEnumArray;
use crate::asc::native_types::r#enum::EnumPayload;
use crate::asc::native_types::string::AscString;
use crate::asc::native_types::Uint8Array;
use crate::bignumber::bigint::BigInt;
use crate::impl_asc_type_enum;

pub type AscH256 = Uint8Array;
pub type AscH2048 = Uint8Array;
pub type AscBigInt = Uint8Array;
pub type AscAddress = Uint8Array;
pub type AscH160 = Uint8Array;

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EthereumValueKind {
    Address,
    FixedBytes,
    Bytes,
    Int,
    Uint,
    Bool,
    String,
    FixedArray,
    Array,
    Tuple,
}

impl EthereumValueKind {
    pub(crate) fn get_kind(token: &ethabi::Token) -> Self {
        match token {
            ethabi::Token::Address(_) => EthereumValueKind::Address,
            ethabi::Token::FixedBytes(_) => EthereumValueKind::FixedBytes,
            ethabi::Token::Bytes(_) => EthereumValueKind::Bytes,
            ethabi::Token::Int(_) => EthereumValueKind::Int,
            ethabi::Token::Uint(_) => EthereumValueKind::Uint,
            ethabi::Token::Bool(_) => EthereumValueKind::Bool,
            ethabi::Token::String(_) => EthereumValueKind::String,
            ethabi::Token::FixedArray(_) => EthereumValueKind::FixedArray,
            ethabi::Token::Array(_) => EthereumValueKind::Array,
            ethabi::Token::Tuple(_) => EthereumValueKind::Tuple,
        }
    }
}

impl_asc_type_enum!(
    EthereumValueKind;
    Address => 0,
    FixedBytes => 1,
    Bytes => 2,
    Int => 3,
    Uint => 4,
    Bool => 5,
    String => 6,
    FixedArray => 7,
    Array => 8,
    Tuple => 9
);

impl Default for EthereumValueKind {
    fn default() -> Self {
        EthereumValueKind::Address
    }
}

impl AscValue for EthereumValueKind {}

impl AscIndexId for Array<AscPtr<AscEnum<EthereumValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEthereumValue;
}

impl ToAscObj<Uint8Array> for web3::H160 {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<Uint8Array, AscError> {
        self.0.to_asc_obj(heap)
    }
}

impl FromAscObj<Uint8Array> for web3::H160 {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: Uint8Array,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let data = <[u8; 20]>::from_asc_obj(typed_array, heap, depth)?;
        Ok(Self(data))
    }
}

impl FromAscObj<Uint8Array> for web3::H256 {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        typed_array: Uint8Array,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let data = <[u8; 32]>::from_asc_obj(typed_array, heap, depth)?;
        Ok(Self(data))
    }
}

impl ToAscObj<Uint8Array> for web3::H256 {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<Uint8Array, AscError> {
        self.0.to_asc_obj(heap)
    }
}

impl ToAscObj<AscBigInt> for web3::U128 {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscBigInt, AscError> {
        let mut bytes: [u8; 16] = [0; 16];
        self.to_little_endian(&mut bytes);
        bytes.to_asc_obj(heap)
    }
}

impl ToAscObj<AscEnum<EthereumValueKind>> for ethabi::Token {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEnum<EthereumValueKind>, AscError> {
        use ethabi::Token::*;

        let kind = EthereumValueKind::get_kind(self);
        let payload = match self {
            Address(address) => asc_new::<AscAddress, _, _>(heap, address)?.to_payload(),
            FixedBytes(bytes) | Bytes(bytes) => {
                asc_new::<Uint8Array, _, _>(heap, &**bytes)?.to_payload()
            }
            Int(uint) => {
                let n = BigInt::from_signed_u256(uint);
                asc_new(heap, &n)?.to_payload()
            }
            Uint(uint) => {
                let n = BigInt::from_unsigned_u256(uint);
                asc_new(heap, &n)?.to_payload()
            }
            Bool(b) => *b as u64,
            String(string) => asc_new(heap, &**string)?.to_payload(),
            FixedArray(tokens) | Array(tokens) => asc_new(heap, &**tokens)?.to_payload(),
            Tuple(tokens) => asc_new(heap, &**tokens)?.to_payload(),
        };

        Ok(AscEnum {
            kind,
            _padding: 0,
            payload: EnumPayload(payload),
        })
    }
}

impl FromAscObj<AscEnum<EthereumValueKind>> for ethabi::Token {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_enum: AscEnum<EthereumValueKind>,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        use ethabi::Token;

        let payload = asc_enum.payload;
        Ok(match asc_enum.kind {
            EthereumValueKind::Bool => Token::Bool(bool::from(payload)),
            EthereumValueKind::Address => {
                let ptr: AscPtr<AscAddress> = AscPtr::from(payload);
                Token::Address(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::FixedBytes => {
                let ptr: AscPtr<Uint8Array> = AscPtr::from(payload);
                Token::FixedBytes(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::Bytes => {
                let ptr: AscPtr<Uint8Array> = AscPtr::from(payload);
                Token::Bytes(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::Int => {
                let ptr: AscPtr<AscBigInt> = AscPtr::from(payload);
                let n: BigInt = asc_get(heap, ptr, depth)?;
                Token::Int(n.to_signed_u256())
            }
            EthereumValueKind::Uint => {
                let ptr: AscPtr<AscBigInt> = AscPtr::from(payload);
                let n: BigInt = asc_get(heap, ptr, depth)?;
                Token::Uint(n.to_unsigned_u256())
            }
            EthereumValueKind::String => {
                let ptr: AscPtr<AscString> = AscPtr::from(payload);
                Token::String(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::FixedArray => {
                let ptr: AscEnumArray<EthereumValueKind> = AscPtr::from(payload);
                Token::FixedArray(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::Array => {
                let ptr: AscEnumArray<EthereumValueKind> = AscPtr::from(payload);
                Token::Array(asc_get(heap, ptr, depth)?)
            }
            EthereumValueKind::Tuple => {
                let ptr: AscEnumArray<EthereumValueKind> = AscPtr::from(payload);
                Token::Tuple(asc_get(heap, ptr, depth)?)
            }
        })
    }
}
