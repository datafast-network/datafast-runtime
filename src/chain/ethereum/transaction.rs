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
use crate::asc::native_types::Uint8Array;
use crate::bignumber::bigint::BigInt;
use crate::impl_asc_type_struct;
use ethabi::Bytes;
use semver::Version;
use web3::types::Transaction;
use web3::types::H160;
use web3::types::H256;
use web3::types::U128;
use web3::types::U256;

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

pub struct AscTransactionArray(Array<AscPtr<AscEthereumTransaction>>);

impl AscType for AscTransactionArray {
    fn to_asc_bytes(&self) -> Result<Vec<u8>, AscError> {
        self.0.to_asc_bytes()
    }

    fn from_asc_bytes(asc_obj: &[u8], api_version: &Version) -> Result<Self, AscError> {
        Ok(Self(Array::from_asc_bytes(asc_obj, api_version)?))
    }
}

impl ToAscObj<AscTransactionArray> for Vec<EthereumTransactionData> {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
    ) -> Result<AscTransactionArray, AscError> {
        let txs = self
            .iter()
            .map(|tx| asc_new(heap, &tx))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AscTransactionArray(Array::new(&txs, heap)?))
    }
}

impl AscIndexId for AscTransactionArray {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::ArrayTransactions;
}

impl FromAscObj<AscTransactionArray> for Vec<EthereumTransactionData> {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        obj: AscTransactionArray,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        let txs: Vec<AscPtr<AscEthereumTransaction>> = obj.0.to_vec(heap)?;
        txs.into_iter()
            .map(|tx| asc_get(heap, tx, depth))
            .collect::<Result<Vec<_>, _>>()
    }
}
