use super::asc::*;

use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::IndexForAscTypeId;
use crate::impl_asc_type_struct;

#[repr(C)]
pub(crate) struct AscBlock {
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

impl AscIndexId for AscBlock {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumBlock;
}

impl_asc_type_struct!(
    AscBlock;
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

/*
/// Convert bblock data from query store to Asc Block
impl ToAscObj<AscBlock> for BlockFromQueryStore {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
        gas: &GasCounter,
    ) -> Result<AscBlock, AscError> {
        Ok(AscBlock {
            hash: asc_new(heap, &self.hash, gas)?,
            parent_hash: asc_new(heap, &self.parent_hash, gas)?,
            uncles_hash: asc_new(heap, &self.uncles_hash, gas)?,
            author: asc_new(heap, &self.author, gas)?,
            state_root: asc_new(heap, &self.state_root, gas)?,
            transactions_root: asc_new(heap, &self.transactions_root, gas)?,
            receipts_root: asc_new(heap, &self.receipts_root, gas)?,
            number: asc_new(heap, &BigInt::from(self.number), gas)?,
            gas_used: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_used), gas)?,
            gas_limit: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_limit), gas)?,
            timestamp: asc_new(heap, &BigInt::from_unsigned_u256(&self.timestamp), gas)?,
            difficulty: asc_new(heap, &BigInt::from_unsigned_u256(&self.difficulty), gas)?,
            total_difficulty: asc_new(
                heap,
                &BigInt::from_unsigned_u256(&self.total_difficulty),
                gas,
            )?,
            size: self
                .size
                .map(|size| asc_new(heap, &BigInt::from_unsigned_u256(&size), gas))
                .unwrap_or(Ok(AscPtr::null()))?,
            base_fee_per_block: self
                .base_fee_per_gas
                .map(|base_fee| asc_new(heap, &BigInt::from_unsigned_u256(&base_fee), gas))
                .unwrap_or(Ok(AscPtr::null()))?,
        })
    }
}
*/
