use super::asc::*;
use crate::asc::base::asc_get;
use crate::asc::base::asc_get_optional;
use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::FromAscObj;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::bignumber::bigint::BigInt;
use crate::chain::ethereum::log::AscLogArray;
use crate::chain::ethereum::transaction::AscTransactionArray;
use crate::chain::ethereum::transaction::EthereumTransactionData;
use crate::impl_asc_type_struct;
use semver::Version;
use web3::types::Block;
use web3::types::Log;
use web3::types::H160;
use web3::types::H256;
use web3::types::U256;
use web3::types::U64;

#[repr(C)]
pub struct AscEthereumBlock {
    pub hash: AscPtr<AscH256>,
    pub parent_hash: AscPtr<AscH256>,
    pub uncles_hash: AscPtr<AscH256>,
    pub author: AscPtr<AscH160>,
    pub state_root: AscPtr<AscH256>,
    pub transactions_root: AscPtr<AscH256>,
    pub receipts_root: AscPtr<AscH256>,
    pub number: AscPtr<AscH256>,
    pub gas_used: AscPtr<AscBigInt>,
    pub gas_limit: AscPtr<AscBigInt>,
    pub timestamp: AscPtr<AscBigInt>,
    pub difficulty: AscPtr<AscBigInt>,
    pub total_difficulty: AscPtr<AscBigInt>,
    pub size: AscPtr<AscBigInt>,
    pub base_fee_per_block: AscPtr<AscBigInt>,
}

impl AscIndexId for AscEthereumBlock {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumBlock;
}

impl_asc_type_struct!(
    AscEthereumBlock;
    hash => AscPtr<AscH256>,
    parent_hash => AscPtr<AscH256>,
    uncles_hash => AscPtr<AscH256>,
    author => AscPtr<AscH160>,
    state_root => AscPtr<AscH256>,
    transactions_root => AscPtr<AscH256>,
    receipts_root => AscPtr<AscH256>,
    number => AscPtr<AscH256>,
    gas_used => AscPtr<AscBigInt>,
    gas_limit => AscPtr<AscBigInt>,
    timestamp => AscPtr<AscBigInt>,
    difficulty => AscPtr<AscBigInt>,
    total_difficulty => AscPtr<AscBigInt>,
    size => AscPtr<AscBigInt>,
    base_fee_per_block => AscPtr<AscBigInt>
);

#[derive(Clone, Debug, Default)]
pub struct EthereumBlockData {
    pub hash: H256,
    pub parent_hash: H256,
    pub uncles_hash: H256,
    pub author: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub number: U64,
    pub gas_used: U256,
    pub gas_limit: U256,
    pub timestamp: U256,
    pub difficulty: U256,
    pub total_difficulty: U256,
    pub size: Option<U256>,
    pub base_fee_per_gas: Option<U256>,
}

impl<'a, T> From<&'a Block<T>> for EthereumBlockData {
    fn from(block: &'a Block<T>) -> EthereumBlockData {
        EthereumBlockData {
            hash: block.hash.unwrap(),
            parent_hash: block.parent_hash,
            uncles_hash: block.uncles_hash,
            author: block.author,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            receipts_root: block.receipts_root,
            number: block.number.unwrap(),
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
            timestamp: block.timestamp,
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty.unwrap_or_default(),
            size: block.size,
            base_fee_per_gas: block.base_fee_per_gas,
        }
    }
}

impl ToAscObj<AscEthereumBlock> for EthereumBlockData {
    fn to_asc_obj<H: AscHeap + ?Sized>(&self, heap: &mut H) -> Result<AscEthereumBlock, AscError> {
        Ok(AscEthereumBlock {
            hash: asc_new(heap, &self.hash)?,
            parent_hash: asc_new(heap, &self.parent_hash)?,
            uncles_hash: asc_new(heap, &self.uncles_hash)?,
            author: asc_new(heap, &self.author)?,
            state_root: asc_new(heap, &self.state_root)?,
            transactions_root: asc_new(heap, &self.transactions_root)?,
            receipts_root: asc_new(heap, &self.receipts_root)?,
            number: asc_new(heap, &BigInt::from(self.number))?,
            gas_used: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_used))?,
            gas_limit: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_limit))?,
            timestamp: asc_new(heap, &BigInt::from_unsigned_u256(&self.timestamp))?,
            difficulty: asc_new(heap, &BigInt::from_unsigned_u256(&self.difficulty))?,
            total_difficulty: asc_new(heap, &BigInt::from_unsigned_u256(&self.total_difficulty))?,
            size: self
                .size
                .map(|size| asc_new(heap, &BigInt::from_unsigned_u256(&size)))
                .unwrap_or(Ok(AscPtr::null()))?,
            base_fee_per_block: self
                .base_fee_per_gas
                .map(|base_fee| asc_new(heap, &BigInt::from_unsigned_u256(&base_fee)))
                .unwrap_or(Ok(AscPtr::null()))?,
        })
    }
}

impl FromAscObj<AscEthereumBlock> for EthereumBlockData {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscEthereumBlock,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok(Self {
            hash: asc_get(heap, obj.hash, depth)?,
            parent_hash: asc_get(heap, obj.parent_hash, depth)?,
            uncles_hash: asc_get(heap, obj.uncles_hash, depth)?,
            author: asc_get(heap, obj.author, depth)?,
            state_root: asc_get(heap, obj.state_root, depth)?,
            transactions_root: asc_get(heap, obj.transactions_root, depth)?,
            receipts_root: asc_get(heap, obj.receipts_root, depth)?,
            number: asc_get(heap, obj.number, depth)?,
            gas_used: asc_get(heap, obj.gas_used, depth)?,
            gas_limit: asc_get(heap, obj.gas_limit, depth)?,
            timestamp: asc_get(heap, obj.timestamp, depth)?,
            difficulty: asc_get(heap, obj.difficulty, depth)?,
            total_difficulty: asc_get(heap, obj.total_difficulty, depth)?,
            size: asc_get_optional(heap, obj.size, depth)?,
            base_fee_per_gas: asc_get_optional(heap, obj.base_fee_per_block, depth)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct EthereumFullBlock {
    pub number: U64,
    pub hash: H256,
    pub parent_hash: H256,
    pub header: EthereumBlockData,
    pub transactions: Vec<EthereumTransactionData>,
    pub logs: Vec<Log>,
}

#[repr(C)]
pub struct AscEthereumFullBlock {
    pub hash: AscPtr<AscH256>,
    pub parent_hash: AscPtr<AscH256>,
    pub number: AscPtr<AscH256>,
    pub header: AscPtr<AscEthereumBlock>,
    pub transactions: AscPtr<AscTransactionArray>,
    pub logs: AscPtr<AscLogArray>,
}

impl AscIndexId for AscEthereumFullBlock {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumFullBlock;
}

impl_asc_type_struct!(
    AscEthereumFullBlock;
    hash => AscPtr<AscH256>,
    parent_hash => AscPtr<AscH256>,
    number => AscPtr<AscH256>,
    header => AscPtr<AscEthereumBlock>,
    transactions => AscPtr<AscTransactionArray>,
    logs => AscPtr<AscLogArray>
);

impl FromAscObj<AscEthereumFullBlock> for EthereumFullBlock {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscEthereumFullBlock,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok(Self {
            hash: asc_get(heap, obj.hash, depth)?,
            parent_hash: asc_get(heap, obj.parent_hash, depth)?,
            number: asc_get(heap, obj.number, depth)?,
            header: asc_get(heap, obj.header, depth)?,
            transactions: asc_get(heap, obj.transactions, depth)?,
            logs: asc_get(heap, obj.logs, depth)?,
        })
    }
}

impl From<(EthereumBlockData, Vec<EthereumTransactionData>, Vec<Log>)> for EthereumFullBlock {
    fn from(
        (header, transactions, logs): (EthereumBlockData, Vec<EthereumTransactionData>, Vec<Log>),
    ) -> Self {
        Self {
            number: header.number,
            hash: header.hash,
            parent_hash: header.parent_hash,
            header,
            transactions,
            logs,
        }
    }
}

impl EthereumFullBlock {
    pub fn transaction_for_log(&self, log: &Log) -> Option<EthereumTransactionData> {
        log.transaction_hash
            .and_then(|hash| self.transactions.iter().find(|tx| tx.hash == hash))
            .cloned()
    }
}
