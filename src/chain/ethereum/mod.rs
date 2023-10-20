mod asc;
use asc::*;

use crate::asc::asc_base::{
    asc_new, AscHeap, AscIndexId, AscPtr, AscType, AscValue, IndexForAscTypeId, ToAscObj,
};

use crate::asc::asc_types::{Array, AscEnum, AscString, Uint8Array};
use crate::asc::errors::AscError;
#[repr(C)]
pub struct Block {
    pub hash: AscPtr<AscH256>,
    pub parent_hash: AscPtr<AscH256>,
    pub uncles_hash: AscPtr<AscH256>,
    pub author: AscPtr<AscH160>,
    pub state_root: AscPtr<AscH256>,
    pub transactions_root: AscPtr<AscH256>,
    pub receipts_root: AscPtr<AscH256>,
    pub number: AscPtr<AscBigInt>,
    pub gas_used: AscPtr<AscBigInt>,
    pub gas_limit: AscPtr<AscBigInt>,
    pub timestamp: AscPtr<AscBigInt>,
    pub difficulty: AscPtr<AscBigInt>,
    pub total_difficulty: AscPtr<AscBigInt>,
    pub size: AscPtr<AscBigInt>,
    pub base_fee_per_block: AscPtr<AscBigInt>,
}

#[repr(C)]
pub(crate) struct AscEthereumTransaction {
    pub hash: AscPtr<AscH256>,
    pub index: AscPtr<AscBigInt>,
    pub from: AscPtr<AscH160>,
    pub to: AscPtr<AscH160>,
    pub value: AscPtr<AscBigInt>,
    pub gas_limit: AscPtr<AscBigInt>,
    pub gas_price: AscPtr<AscBigInt>,
    pub input: AscPtr<Uint8Array>,
    pub nonce: AscPtr<AscBigInt>,
}

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

impl AscType for EthereumValueKind {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        let discriminant: u32 = match self {
            EthereumValueKind::Address => 0,
            EthereumValueKind::FixedBytes => 1,
            EthereumValueKind::Bytes => 2,
            EthereumValueKind::Int => 3,
            EthereumValueKind::Uint => 4,
            EthereumValueKind::Bool => 5,
            EthereumValueKind::String => 6,
            EthereumValueKind::FixedArray => 7,
            EthereumValueKind::Array => 8,
            EthereumValueKind::Tuple => 9,
        };
        discriminant.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        let u32_bytes = ::std::convert::TryFrom::try_from(asc_obj)
            .map_err(|_| AscError::Plain("invalid Kind".to_string()))?;
        let discriminant = u32::from_le_bytes(u32_bytes);
        match discriminant {
            0 => Ok(EthereumValueKind::Address),
            1 => Ok(EthereumValueKind::FixedBytes),
            2 => Ok(EthereumValueKind::Bytes),
            3 => Ok(EthereumValueKind::Int),
            4 => Ok(EthereumValueKind::Uint),
            5 => Ok(EthereumValueKind::Bool),
            6 => Ok(EthereumValueKind::String),
            7 => Ok(EthereumValueKind::FixedArray),
            8 => Ok(EthereumValueKind::Array),
            9 => Ok(EthereumValueKind::Tuple),
            _ => Err(AscError::Plain("invalid Kind".to_string())),
        }
    }
}

impl Default for EthereumValueKind {
    fn default() -> Self {
        EthereumValueKind::Address
    }
}

impl AscValue for EthereumValueKind {}

impl AscIndexId for Array<AscPtr<AscEnum<EthereumValueKind>>> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEthereumValue;
}

//LogParam for ASC
#[repr(C)]
pub(crate) struct AscLogParam {
    pub name: AscPtr<AscString>,
    pub value: AscPtr<AscEnum<EthereumValueKind>>,
}

pub struct AscLogParamArray(Array<AscPtr<AscLogParam>>);
impl AscIndexId for AscLogParam {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EventParam;
}

impl AscIndexId for AscEnum<EthereumValueKind> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumValue;
}

impl AscType for AscLogParam {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        todo!()
    }

    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        todo!()
    }
}

impl AscType for AscLogParamArray {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }
    fn from_asc_bytes(asc_obj: &[u8]) -> Result<Self, AscError> {
        Ok(Self(Array::from_asc_bytes(asc_obj)?))
    }
}

impl ToAscObj<AscLogParamArray> for Vec<ethabi::LogParam> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscLogParamArray, AscError> {
        let content: Result<Vec<_>, _> = self.iter().map(|x| asc_new(heap, x)).collect();
        let content = content?;
        Ok(AscLogParamArray(Array::new(&content, heap)?))
    }
}

impl ToAscObj<AscLogParam> for ethabi::LogParam {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscLogParam, AscError> {
        Ok(AscLogParam {
            name: asc_new(heap, self.name.as_str())?,
            value: asc_new(heap, &self.value)?,
        })
    }
}

impl AscIndexId for AscLogParamArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEventParam;
}

#[repr(C)]
pub(crate) struct AscEthereumEvent<T, B>
where
    T: AscType,
    B: AscType,
{
    pub address: AscPtr<AscAddress>,
    pub log_index: AscPtr<AscBigInt>,
    pub transaction_log_index: AscPtr<AscBigInt>,
    pub log_type: AscPtr<AscString>,
    pub block: AscPtr<B>,
    pub transaction: AscPtr<T>,
    pub params: AscPtr<AscLogParamArray>,
}
