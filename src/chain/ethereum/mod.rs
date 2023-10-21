mod asc;

use asc::*;

use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::AscValue;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;

use crate::impl_asc_type_enum;
use crate::impl_asc_type_struct;

use crate::asc::errors::AscError;
use crate::asc::native_types::array::Array;
use crate::asc::native_types::r#enum::AscEnum;
use crate::asc::native_types::string::AscString;
use crate::asc::native_types::Uint8Array;

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

impl_asc_type_struct!(
    Block;
    hash => AscPtr<AscH256>,
    parent_hash => AscPtr<AscH256>,
    uncles_hash => AscPtr<AscH256>,
    author => AscPtr<AscH160>,
    state_root => AscPtr<AscH256>,
    transactions_root => AscPtr<AscH256>,
    receipts_root => AscPtr<AscH256>,
    number => AscPtr<AscBigInt>,
    gas_used => AscPtr<AscBigInt>,
    gas_limit => AscPtr<AscBigInt>,
    timestamp => AscPtr<AscBigInt>,
    difficulty => AscPtr<AscBigInt>,
    total_difficulty => AscPtr<AscBigInt>,
    size => AscPtr<AscBigInt>,
    base_fee_per_block => AscPtr<AscBigInt>
);

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

impl_asc_type_struct!(
    AscEthereumTransaction;
    hash => AscPtr<AscH256>,
    index => AscPtr<AscBigInt>,
    from => AscPtr<AscH160>,
    to => AscPtr<AscH160>,
    value => AscPtr<AscBigInt>,
    gas_limit => AscPtr<AscBigInt>,
    gas_price => AscPtr<AscBigInt>,
    input => AscPtr<Uint8Array>,
    nonce => AscPtr<AscBigInt>
);

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

#[repr(C)]
pub struct AscLogParam {
    pub name: AscPtr<AscString>,
    pub value: AscPtr<AscEnum<EthereumValueKind>>,
}

impl_asc_type_struct!(
    AscLogParam;
    name => AscPtr<AscString>,
    value => AscPtr<AscEnum<EthereumValueKind>>
);

pub struct AscLogParamArray(Array<AscPtr<AscLogParam>>);
impl AscIndexId for AscLogParam {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EventParam;
}

impl AscIndexId for AscEnum<EthereumValueKind> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumValue;
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
pub struct AscEthereumEvent<T: AscType, B: AscType> {
    pub address: AscPtr<AscAddress>,
    pub log_index: AscPtr<AscBigInt>,
    pub transaction_log_index: AscPtr<AscBigInt>,
    pub log_type: AscPtr<AscString>,
    pub block: AscPtr<B>,
    pub transaction: AscPtr<T>,
    pub params: AscPtr<AscLogParamArray>,
}

impl_asc_type_struct!(
    AscEthereumEvent<T: AscType, B: AscType>;
    address => AscPtr<AscAddress>,
    log_index => AscPtr<AscBigInt>,
    transaction_log_index => AscPtr<AscBigInt>,
    log_type => AscPtr<AscString>,
    block => AscPtr<B>,
    transaction => AscPtr<T>,
    params => AscPtr<AscLogParamArray>
);
