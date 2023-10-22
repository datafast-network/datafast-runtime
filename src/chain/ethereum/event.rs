use super::asc::*;

use super::log::AscLogParamArray;
use crate::asc::base::AscPtr;
use crate::asc::base::AscType;
use crate::asc::native_types::string::AscString;
use crate::impl_asc_type_struct;

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
