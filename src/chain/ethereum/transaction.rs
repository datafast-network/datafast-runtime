use super::asc::*;

use crate::asc::base::AscPtr;
use crate::asc::native_types::Uint8Array;
use crate::impl_asc_type_struct;

#[repr(C)]
pub(crate) struct Transaction {
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
    Transaction;
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
