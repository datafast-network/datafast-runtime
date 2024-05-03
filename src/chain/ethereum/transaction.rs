use super::asc::*;
use crate::chain::ethereum::block::EthereumBlockData;
use crate::chain::ethereum::log::AscLogArray;
use crate::errors::AscError;
use crate::impl_asc_type_struct;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_get_optional;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::FromAscObj;
use crate::runtime::asc::base::IndexForAscTypeId;
use crate::runtime::asc::base::ToAscObj;
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::Uint8Array;
use crate::runtime::bignumber::bigint::BigInt;
use ethabi::Bytes;
use semver::Version;
use web3::types::Log;
use web3::types::Transaction;
use web3::types::TransactionReceipt;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;
use web3::types::U64;

#[repr(C)]
pub struct AscEthereumTransaction {
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

impl AscIndexId for AscEthereumTransaction {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumTransaction;
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

#[derive(Clone, Debug, Default)]
pub struct EthereumTransactionData {
    pub hash: H256,
    pub index: U128,
    pub from: H160,
    pub to: Option<H160>,
    pub value: U256,
    pub gas_limit: U256,
    pub gas_price: U256,
    pub input: Bytes,
    pub nonce: U256,
}

impl From<&'_ Transaction> for EthereumTransactionData {
    fn from(tx: &Transaction) -> EthereumTransactionData {
        // unwrap: this is always `Some` for txns that have been mined
        //         (see https://github.com/tomusdrw/rust-web3/pull/407)
        let from = tx.from.unwrap();
        EthereumTransactionData {
            hash: tx.hash,
            index: tx.transaction_index.unwrap().as_u64().into(),
            from,
            to: tx.to,
            value: tx.value,
            gas_limit: tx.gas,
            gas_price: tx.gas_price.unwrap_or(U256::zero()), // EIP-1559 made this optional.
            input: tx.input.0.clone(),
            nonce: tx.nonce,
        }
    }
}

impl ToAscObj<AscEthereumTransaction> for EthereumTransactionData {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscEthereumTransaction, AscError> {
        Ok(AscEthereumTransaction {
            hash: asc_new(heap, &self.hash)?,
            index: asc_new(heap, &BigInt::from_unsigned_u128(self.index))?,
            from: asc_new(heap, &self.from)?,
            to: self
                .to
                .map(|to| asc_new(heap, &to))
                .unwrap_or(Ok(AscPtr::null()))?,
            value: asc_new(heap, &BigInt::from_unsigned_u256(&self.value))?,
            gas_limit: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_limit))?,
            gas_price: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_price))?,
            input: asc_new(heap, &*self.input)?,
            nonce: asc_new(heap, &BigInt::from_unsigned_u256(&self.nonce))?,
        })
    }
}

impl FromAscObj<AscEthereumTransaction> for EthereumTransactionData {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscEthereumTransaction,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        Ok(EthereumTransactionData {
            hash: asc_get(heap, obj.hash, 0)?,
            index: asc_get(heap, obj.index, 0)?,
            from: asc_get(heap, obj.from, 0)?,
            to: asc_get_optional(heap, obj.to, 0)?,
            value: asc_get(heap, obj.value, 0)?,
            gas_limit: asc_get(heap, obj.gas_limit, 0)?,
            gas_price: asc_get(heap, obj.gas_price, 0)?,
            input: asc_get(heap, obj.input, 0)?,
            nonce: asc_get(heap, obj.nonce, 0)?,
        })
    }
}

pub type AscTransactionArray = Array<AscPtr<AscEthereumTransaction>>;

impl ToAscObj<AscTransactionArray> for Vec<EthereumTransactionData> {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscTransactionArray, AscError> {
        let txs = self
            .iter()
            .map(|tx| asc_new(heap, &tx))
            .collect::<Result<Vec<_>, _>>()?;
        AscTransactionArray::new(&txs, heap)
    }
}

impl AscIndexId for AscTransactionArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayEthereumTransaction;
}

#[repr(C)]
pub struct AscTransactionReceipt {
    pub transaction_hash: AscPtr<Uint8Array>,
    pub transaction_index: AscPtr<AscBigInt>,
    pub block_hash: AscPtr<Uint8Array>,
    pub block_number: AscPtr<AscBigInt>,
    pub cumulative_gas_used: AscPtr<AscBigInt>,
    pub gas_used: AscPtr<AscBigInt>,
    pub contract_address: AscPtr<AscH160>,
    pub logs: AscPtr<AscLogArray>,
    pub status: AscPtr<AscBigInt>,
    pub root: AscPtr<Uint8Array>,
    pub logs_bloom: AscPtr<Uint8Array>,
}

impl AscIndexId for AscTransactionReceipt {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TransactionReceipt;
}

impl_asc_type_struct!(
    AscTransactionReceipt;
    transaction_hash => AscPtr<Uint8Array>,
    transaction_index => AscPtr<AscBigInt>,
    block_hash => AscPtr<Uint8Array>,
    block_number => AscPtr<AscBigInt>,
    cumulative_gas_used => AscPtr<AscBigInt>,
    gas_used => AscPtr<AscBigInt>,
    contract_address => AscPtr<AscH160>,
    logs => AscPtr<AscLogArray>,
    status => AscPtr<AscBigInt>,
    root => AscPtr<Uint8Array>,
    logs_bloom => AscPtr<Uint8Array>
);

#[derive(Clone, Debug, Default)]
pub struct EthereumTransactionReceipt {
    pub transaction_hash: H256,
    pub transaction_index: U64,
    pub block_hash: H256,
    pub block_number: U64,
    pub cumulative_gas_used: U256,
    pub gas_used: Option<U256>,
    pub contract_address: Option<H160>,
    pub logs: Vec<Log>,
    pub status: Option<U64>,
    pub root: Option<H256>,
    pub logs_bloom: Bytes,
}

impl From<TransactionReceipt> for EthereumTransactionReceipt {
    fn from(receipt: TransactionReceipt) -> EthereumTransactionReceipt {
        EthereumTransactionReceipt {
            transaction_hash: receipt.transaction_hash,
            transaction_index: receipt.transaction_index,
            block_hash: receipt.block_hash.unwrap(),
            block_number: receipt.block_number.unwrap().as_u64().into(),
            cumulative_gas_used: receipt.cumulative_gas_used,
            gas_used: receipt.gas_used,
            contract_address: receipt.contract_address,
            logs: receipt.logs,
            status: receipt.status,
            root: receipt.root,
            logs_bloom: receipt.logs_bloom.as_bytes().to_vec(),
        }
    }
}

impl FromAscObj<AscTransactionReceipt> for EthereumTransactionReceipt {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscTransactionReceipt,
        heap: &H,
        _depth: usize,
    ) -> Result<Self, AscError> {
        Ok(EthereumTransactionReceipt {
            transaction_hash: asc_get(heap, obj.transaction_hash, 0)?,
            transaction_index: asc_get(heap, obj.transaction_index, 0)?,
            block_hash: asc_get(heap, obj.block_hash, 0)?,
            block_number: asc_get(heap, obj.block_number, 0)?,
            cumulative_gas_used: asc_get(heap, obj.cumulative_gas_used, 0)?,
            gas_used: asc_get_optional(heap, obj.gas_used, 0)?,
            contract_address: asc_get_optional(heap, obj.contract_address, 0)?,
            logs: asc_get(heap, obj.logs, 0)?,
            status: asc_get_optional(heap, obj.status, 0)?,
            root: asc_get_optional(heap, obj.root, 0)?,
            logs_bloom: asc_get(heap, obj.logs_bloom, 0)?,
        })
    }
}

impl ToAscObj<AscTransactionReceipt> for EthereumTransactionReceipt {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscTransactionReceipt, AscError> {
        Ok(AscTransactionReceipt {
            transaction_hash: asc_new(heap, &self.transaction_hash)?,
            transaction_index: asc_new(heap, &BigInt::from(self.transaction_index))?,
            block_hash: asc_new(heap, &self.block_hash)?,
            block_number: asc_new(heap, &BigInt::from(self.block_number))?,
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
            logs_bloom: asc_new(heap, &*self.logs_bloom)?,
        })
    }
}

impl From<(&'_ EthereumBlockData, EthereumTransactionData, &'_ Vec<Log>)>
    for EthereumTransactionReceipt
{
    fn from(
        (block, tx, logs): (&EthereumBlockData, EthereumTransactionData, &Vec<Log>),
    ) -> EthereumTransactionReceipt {
        let mut logs_of_tx = logs
            .iter()
            .filter_map(|log| {
                if let Some(hash) = &log.transaction_hash {
                    if hash.eq(&tx.hash) {
                        return Some(log.clone());
                    }
                }
                None
            })
            .collect::<Vec<Log>>();

        logs_of_tx.sort_by_key(|log| log.log_index.unwrap());

        EthereumTransactionReceipt {
            transaction_hash: tx.hash,
            transaction_index: tx.index.as_u64().into(),
            block_hash: block.hash,
            block_number: block.number,
            cumulative_gas_used: U256::zero(),
            gas_used: None,
            contract_address: None,
            logs: logs_of_tx,
            status: None,
            root: None,
            logs_bloom: Bytes::default(),
        }
    }
}
