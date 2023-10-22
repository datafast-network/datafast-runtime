use super::asc::*;
use ethabi::Bytes;
use web3::types::Transaction;
use web3::types::TransactionReceipt;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;

use crate::asc::base::asc_new;
use crate::asc::base::AscHeap;
use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::base::ToAscObj;
use crate::asc::errors::AscError;
use crate::asc::native_types::Uint8Array;
use crate::bignumber::bigint::BigInt;
use crate::chain::ethereum::log::AscLogArray;
use crate::impl_asc_type_struct;

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

/*
/// Convert to Asc Transaction from Query Store
impl ToAscObj<AscEthereumTransaction> for EthereumTransactionData {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
        gas: &GasCounter,
    ) -> Result<AscEthereumTransaction, HostExportError> {
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
}*/

#[derive(Clone, Debug)]
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

impl AscIndexId for AscEthereumTransactionReceipt {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::TransactionReceipt;
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
