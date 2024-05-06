use df_types::chain::ethereum::asc::AscAddress;
use df_types::chain::ethereum::asc::EthereumValueKind;
use df_types::errors::AscError;
use crate::impl_asc_type_struct;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::AscHeap;
use crate::runtime::asc::base::AscIndexId;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::base::FromAscObj;
use crate::runtime::asc::base::IndexForAscTypeId;
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::r#enum::AscEnum;
use crate::runtime::asc::native_types::string::AscString;
use ethabi::Function;
use ethabi::Token;
use semver::Version;
use df_types::web3::types::Address;

#[repr(C)]
pub struct AscUnresolvedContractCallV4 {
    pub contract_name: AscPtr<AscString>,
    pub contract_address: AscPtr<AscAddress>,
    pub function_name: AscPtr<AscString>,
    pub function_signature: AscPtr<AscString>,
    pub function_args: AscPtr<Array<AscPtr<AscEnum<EthereumValueKind>>>>,
}

impl_asc_type_struct!(
    AscUnresolvedContractCallV4;
    contract_name => AscPtr<AscString>,
    contract_address => AscPtr<AscAddress>,
    function_name => AscPtr<AscString>,
    function_signature => AscPtr<AscString>,
    function_args => AscPtr<Array<AscPtr<AscEnum<EthereumValueKind>>>>
);

impl AscIndexId for AscUnresolvedContractCallV4 {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::SmartContractCall;
}

impl FromAscObj<AscUnresolvedContractCallV4> for UnresolvedContractCall {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_call: AscUnresolvedContractCallV4,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok(UnresolvedContractCall {
            contract_name: asc_get(heap, asc_call.contract_name, depth)?,
            contract_address: asc_get(heap, asc_call.contract_address, depth)?,
            function_name: asc_get(heap, asc_call.function_name, depth)?,
            function_signature: Some(asc_get(heap, asc_call.function_signature, depth)?),
            function_args: asc_get(heap, asc_call.function_args, depth)?,
        })
    }
}

#[repr(C)]
pub struct AscUnresolvedContractCall {
    pub contract_name: AscPtr<AscString>,
    pub contract_address: AscPtr<AscAddress>,
    pub function_name: AscPtr<AscString>,
    pub function_args: AscPtr<Array<AscPtr<AscEnum<EthereumValueKind>>>>,
}

impl_asc_type_struct!(
    AscUnresolvedContractCall;
    contract_name => AscPtr<AscString>,
    contract_address => AscPtr<AscAddress>,
    function_name => AscPtr<AscString>,
    function_args => AscPtr<Array<AscPtr<AscEnum<EthereumValueKind>>>>
);

#[derive(Clone, Debug, PartialEq)]
pub struct UnresolvedContractCall {
    pub contract_name: String,
    pub contract_address: Address,
    pub function_name: String,
    pub function_signature: Option<String>,
    pub function_args: Vec<Token>,
}

impl Eq for UnresolvedContractCall {}
impl std::hash::Hash for UnresolvedContractCall {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.contract_name.hash(state);
        self.contract_address.hash(state);
        self.function_name.hash(state);
        self.function_signature.hash(state);
        self.function_args
            .iter()
            .for_each(|t| t.clone().into_bytes().hash(state));
    }
}

impl FromAscObj<AscUnresolvedContractCall> for UnresolvedContractCall {
    fn from_asc_obj<H: AscHeap + ?Sized>(
        asc_call: AscUnresolvedContractCall,
        heap: &H,
        depth: usize,
    ) -> Result<Self, AscError> {
        Ok(UnresolvedContractCall {
            contract_name: asc_get(heap, asc_call.contract_name, depth)?,
            contract_address: asc_get(heap, asc_call.contract_address, depth)?,
            function_name: asc_get(heap, asc_call.function_name, depth)?,
            function_signature: None,
            function_args: asc_get(heap, asc_call.function_args, depth)?,
        })
    }
}

impl AscIndexId for AscUnresolvedContractCall {
    const INDEX_ASC_TYPE_ID: IndexForAscTypeId = IndexForAscTypeId::SmartContractCall;
}

#[derive(Clone, Debug)]
pub struct EthereumContractCall {
    pub address: Address,
    pub function: Function,
    pub args: Vec<Token>,
}
