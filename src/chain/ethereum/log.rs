use super::asc::*;
use crate::asc::base::asc_get;
use crate::asc::base::asc_get_optional;
use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::FromAscObj;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::asc::native_types::array::Array;
use crate::asc::native_types::r#enum::AscEnum;
use crate::asc::native_types::string::AscString;
use crate::asc::native_types::AscWrapped;
use crate::asc::native_types::Uint8Array;
use crate::bignumber::bigint::BigInt;
use crate::impl_asc_type_struct;
use semver::Version;
use web3::types::Log;
use web3::types::H256;

impl ToAscObj<AscLogParam> for ethabi::LogParam {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscLogParam, AscError> {
        Ok(AscLogParam {
            name: asc_new(heap, self.name.as_str())?,
            value: asc_new(heap, &self.value)?,
        })
    }
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

impl AscIndexId for AscLogParam {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EventParam;
}

pub struct AscLogParamArray(Array<AscPtr<AscLogParam>>);

impl AscType for AscLogParamArray {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }
    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        Ok(Self(Array::from_asc_bytes(asc_obj, api_version)?))
    }
}

impl ToAscObj<AscLogParamArray> for Vec<ethabi::LogParam> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscLogParamArray, AscError> {
        let content: Result<Vec<_>, _> = self
            .iter()
            .map(|log_param| asc_new(heap, log_param))
            .collect();
        let content = content?;
        Ok(AscLogParamArray(Array::new(&content, heap)?))
    }
}

impl AscIndexId for AscLogParamArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEventParam;
}

pub struct AscTopicArray(Array<AscPtr<AscH256>>);

impl AscType for AscTopicArray {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        Ok(Self(Array::from_asc_bytes(asc_obj, api_version)?))
    }
}

impl ToAscObj<AscTopicArray> for Vec<H256> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscTopicArray, AscError> {
        let topics = self
            .iter()
            .map(|topic| asc_new(heap, topic))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AscTopicArray(Array::new(&topics, heap)?))
    }
}

impl FromAscObj<AscTopicArray> for Vec<H256> {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscTopicArray,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let topics: Vec<AscPtr<AscH256>> = obj.0.to_vec(heap)?;
        topics
            .into_iter()
            .map(|topic| asc_get(heap, topic, depth))
            .collect::<Result<Vec<_>, _>>()
    }
}

impl AscIndexId for AscTopicArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayH256;
}

#[repr(C)]
pub struct AscEthereumLog {
    pub address: AscPtr<AscAddress>,
    pub topics: AscPtr<AscTopicArray>,
    pub data: AscPtr<Uint8Array>,
    pub block_hash: AscPtr<AscH256>,
    pub block_number: AscPtr<AscH256>,
    pub transaction_hash: AscPtr<AscH256>,
    pub transaction_index: AscPtr<AscBigInt>,
    pub log_index: AscPtr<AscBigInt>,
    pub transaction_log_index: AscPtr<AscBigInt>,
    pub log_type: AscPtr<AscString>,
    pub removed: AscPtr<AscWrapped<bool>>,
}

impl AscIndexId for AscEthereumLog {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::Log;
}

impl_asc_type_struct!(
    AscEthereumLog;
    address => AscPtr<AscAddress>,
    topics => AscPtr<AscTopicArray>,
    data => AscPtr<Uint8Array>,
    block_hash => AscPtr<AscH256>,
    block_number => AscPtr<AscH256>,
    transaction_hash => AscPtr<AscH256>,
    transaction_index => AscPtr<AscBigInt>,
    log_index => AscPtr<AscBigInt>,
    transaction_log_index => AscPtr<AscBigInt>,
    log_type => AscPtr<AscString>,
    removed => AscPtr<AscWrapped<bool>>
);

pub struct AscLogArray(Array<AscPtr<AscEthereumLog>>);

impl AscType for AscLogArray {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        Ok(Self(Array::from_asc_bytes(asc_obj, api_version)?))
    }
}

impl ToAscObj<AscEthereumLog> for Log {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscEthereumLog, AscError> {
        Ok(AscEthereumLog {
            address: asc_new(heap, &self.address)?,
            topics: asc_new(heap, &self.topics)?,
            data: asc_new(heap, self.data.0.as_slice())?,
            block_hash: self
                .block_hash
                .map(|block_hash| asc_new(heap, &block_hash))
                .unwrap_or(Ok(AscPtr::null()))?,
            block_number: self
                .block_number
                .map(|block_number| asc_new(heap, &BigInt::from(block_number)))
                .unwrap_or(Ok(AscPtr::null()))?,
            transaction_hash: self
                .transaction_hash
                .map(|txn_hash| asc_new(heap, &txn_hash))
                .unwrap_or(Ok(AscPtr::null()))?,
            transaction_index: self
                .transaction_index
                .map(|txn_index| asc_new(heap, &BigInt::from(txn_index)))
                .unwrap_or(Ok(AscPtr::null()))?,
            log_index: self
                .log_index
                .map(|log_index| asc_new(heap, &BigInt::from_unsigned_u256(&log_index)))
                .unwrap_or(Ok(AscPtr::null()))?,
            transaction_log_index: self
                .transaction_log_index
                .map(|index| asc_new(heap, &BigInt::from_unsigned_u256(&index)))
                .unwrap_or(Ok(AscPtr::null()))?,
            log_type: self
                .log_type
                .as_ref()
                .map(|log_type| asc_new(heap, &log_type))
                .unwrap_or(Ok(AscPtr::null()))?,
            removed: self
                .removed
                .map(|removed| asc_new(heap, &AscWrapped { inner: removed }))
                .unwrap_or(Ok(AscPtr::null()))?,
        })
    }
}

impl ToAscObj<AscLogArray> for Vec<Log> {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscLogArray, AscError> {
        let logs = self
            .iter()
            .map(|log| asc_new(heap, &log))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AscLogArray(Array::new(&logs, heap)?))
    }
}

impl AscIndexId for AscLogArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayLog;
}

impl FromAscObj<AscEthereumLog> for Log {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscEthereumLog,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok(Self {
            address: asc_get(heap, obj.address, depth)?,
            topics: asc_get(heap, obj.topics, depth)?,
            data: asc_get(heap, obj.data, depth)?,
            block_hash: asc_get_optional(heap, obj.block_hash, depth)?,
            block_number: asc_get_optional(heap, obj.block_number, depth)?,
            transaction_hash: asc_get_optional(heap, obj.transaction_hash, depth)?,
            transaction_index: asc_get_optional(heap, obj.transaction_index, depth)?,
            log_index: asc_get_optional(heap, obj.log_index, depth)?,
            transaction_log_index: asc_get_optional(heap, obj.transaction_log_index, depth)?,
            log_type: asc_get_optional(heap, obj.log_type, depth)?,
            removed: asc_get_optional(heap, obj.removed, depth)?,
        })
    }
}
