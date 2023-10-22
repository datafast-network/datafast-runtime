use super::asc::*;

use web3::types::TransactionReceipt;

use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::bignumber::bigint::BigInt;
use crate::chain::ethereum::log::AscLogArray;
use crate::impl_asc_type_struct;

#[repr(C)]
pub struct AscEthereumTransactionReceipt {
    pub transaction_hash: AscPtr<AscH256>,
    pub transaction_index: AscPtr<AscBigInt>,
    pub block_hash: AscPtr<AscH256>,
    pub block_number: AscPtr<AscBigInt>,
    pub cumulative_gas_used: AscPtr<AscBigInt>,
    pub gas_used: AscPtr<AscBigInt>,
    pub contract_address: AscPtr<AscAddress>,
    pub logs: AscPtr<AscLogArray>,
    pub status: AscPtr<AscBigInt>,
    pub root: AscPtr<AscH256>,
    pub logs_bloom: AscPtr<AscH2048>,
}

impl_asc_type_struct!(
    AscEthereumTransactionReceipt;
    transaction_hash => AscPtr<AscH256>,
    transaction_index => AscPtr<AscBigInt>,
    block_hash => AscPtr<AscH256>,
    block_number => AscPtr<AscBigInt>,
    cumulative_gas_used => AscPtr<AscBigInt>,
    gas_used => AscPtr<AscBigInt>,
    contract_address => AscPtr<AscAddress>,
    logs => AscPtr<AscLogArray>,
    status => AscPtr<AscBigInt>,
    root => AscPtr<AscH256>,
    logs_bloom => AscPtr<AscH2048>
);

impl AscIndexId for AscEthereumTransactionReceipt {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TransactionReceipt;
}

impl ToAscObj<AscEthereumTransactionReceipt> for &TransactionReceipt {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEthereumTransactionReceipt, AscError> {
        Ok(AscEthereumTransactionReceipt {
            transaction_hash: asc_new(heap, &self.transaction_hash)?,
            transaction_index: asc_new(heap, &BigInt::from(self.transaction_index))?,
            block_hash: self
                .block_hash
                .map(|block_hash| asc_new(heap, &block_hash))
                .unwrap_or(Ok(AscPtr::null()))?,
            block_number: self
                .block_number
                .map(|block_number| asc_new(heap, &BigInt::from(block_number)))
                .unwrap_or(Ok(AscPtr::null()))?,
            cumulative_gas_used: asc_new(
                heap,
                &BigInt::from_unsigned_u256(&self.cumulative_gas_used),
            )?,
            gas_used: self
                .gas_used
                .map(|gas_used| asc_new(heap, &BigInt::from_unsigned_u256(&gas_used)))
                .unwrap_or(Ok(AscPtr::null()))?,
            contract_address: self
                .contract_address
                .map(|contract_address| asc_new(heap, &contract_address))
                .unwrap_or(Ok(AscPtr::null()))?,
            logs: asc_new(heap, &self.logs)?,
            status: self
                .status
                .map(|status| asc_new(heap, &BigInt::from(status)))
                .unwrap_or(Ok(AscPtr::null()))?,
            root: self
                .root
                .map(|root| asc_new(heap, &root))
                .unwrap_or(Ok(AscPtr::null()))?,
            logs_bloom: asc_new(heap, self.logs_bloom.as_bytes())?,
        })
    }
}
