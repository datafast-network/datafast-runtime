use super::asc::*;

use crate::asc::base::AscIndexId;
use crate::asc::base::AscPtr;
use crate::asc::base::IndexForAscTypeId;
use crate::asc::native_types::Uint8Array;
use crate::impl_asc_type_struct;

#[repr(C)]
pub struct AscTransaction {
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

impl AscIndexId for AscTransaction {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::EthereumTransaction;
}

impl_asc_type_struct!(
    AscTransaction;
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
impl ToAscObj<AscTransaction> for EthereumTransactionData {
    fn to_asc_obj<H: AscHeap + ?Sized>(
        &self,
        heap: &mut H,
        gas: &GasCounter,
    ) -> Result<AscTransaction, HostExportError> {
        Ok(AscTransaction {
            hash: asc_new(heap, &self.hash, gas)?,
            index: asc_new(heap, &BigInt::from_unsigned_u128(self.index), gas)?,
            from: asc_new(heap, &self.from, gas)?,
            to: self
                .to
                .map(|to| asc_new(heap, &to, gas))
                .unwrap_or(Ok(AscPtr::null()))?,
            value: asc_new(heap, &BigInt::from_unsigned_u256(&self.value), gas)?,
            gas_limit: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_limit), gas)?,
            gas_price: asc_new(heap, &BigInt::from_unsigned_u256(&self.gas_price), gas)?,
            input: asc_new(heap, &*self.input, gas)?,
            nonce: asc_new(heap, &BigInt::from_unsigned_u256(&self.nonce), gas)?,
        })
    }
}*/
