use super::asc::*;
use super::block::AscEthereumBlock;
use super::block::EthereumBlockData;
use super::log::AscLogParamArray;
use super::transaction::AscEthereumTransaction;
use super::transaction::EthereumTransactionData;
use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::asc::native_types::string::AscString;
use crate::bignumber::bigint::BigInt;
use crate::impl_asc_type_struct;
use ethabi::LogParam;
use semver::Version;
use web3::types::Address;
use web3::types::U256;

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

impl AscIndexId for AscEthereumEvent<AscEthereumTransaction, AscEthereumBlock> {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumEvent;
}

#[derive(Debug, Clone, Default)]
pub struct EthereumEventData {
    pub address: Address,
    pub log_index: U256,
    pub transaction_log_index: U256,
    pub log_type: Option<String>,
    pub block: EthereumBlockData,
    pub transaction: EthereumTransactionData,
    pub params: Vec<LogParam>,
}

impl<T, B> ToAscObj<AscEthereumEvent<T, B>> for EthereumEventData
where
    T: AscType + AscIndexId,
    B: AscType + AscIndexId,
    EthereumTransactionData: ToAscObj<T>,
    EthereumBlockData: ToAscObj<B>,
{
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEthereumEvent<T, B>, AscError> {
        Ok(AscEthereumEvent {
            address: asc_new(heap, &self.address)?,
            log_index: asc_new(heap, &BigInt::from_unsigned_u256(&self.log_index))?,
            transaction_log_index: asc_new(
                heap,
                &BigInt::from_unsigned_u256(&self.transaction_log_index),
            )?,
            log_type: self
                .log_type
                .clone()
                .map(|log_type| asc_new(heap, &log_type))
                .unwrap_or(Ok(AscPtr::null()))?,
            block: asc_new::<B, EthereumBlockData, _>(heap, &self.block)?,
            transaction: asc_new::<T, EthereumTransactionData, _>(heap, &self.transaction)?,
            params: asc_new(heap, &self.params)?,
        })
    }
}
