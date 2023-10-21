use super::asc::*;

use crate::asc::base::AscPtr;
use crate::impl_asc_type_struct;

#[repr(C)]
pub struct Block {
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

impl_asc_type_struct!(
    Block;
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
