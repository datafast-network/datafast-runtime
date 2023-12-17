use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::asc::native_types::typed_map::AscEntity;
use crate::runtime::asc::native_types::Uint8Array;
use crate::runtime::wasm_host::Env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

/// NOTE: as discussed, due to current filter logic, we can skip this function entirely
pub fn datasource_create(
    _fenv: FunctionEnvMut<Env>,
    _name_ptr: AscPtr<AscString>,
    _params_ptr: AscPtr<Array<AscPtr<AscString>>>,
) -> Result<(), RuntimeError> {
    Ok(())
}

pub fn datasource_create_context(
    _fenv: FunctionEnvMut<Env>,
    _name_ptr: AscPtr<AscString>,
    _params_ptr: AscPtr<Array<AscPtr<AscString>>>,
    _context_ptr: AscPtr<AscEntity>,
) -> Result<(), RuntimeError> {
    todo!()
}

pub fn datasource_address(
    mut fenv: FunctionEnvMut<Env>,
) -> Result<AscPtr<Uint8Array>, RuntimeError> {
    let address = fenv
        .data()
        .address
        .as_ref()
        .map(|a| a.as_bytes().to_vec())
        .unwrap_or(vec![]);
    let address_ptr = asc_new(&mut fenv, address.as_slice())?;
    Ok(address_ptr)
}

pub fn datasource_network(
    mut fenv: FunctionEnvMut<Env>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let network = fenv.data().network.clone();
    let network_ptr = asc_new(&mut fenv, &network).unwrap();
    Ok(network_ptr)
}

pub fn datasource_context(_fenv: FunctionEnvMut<Env>) -> Result<AscPtr<AscEntity>, RuntimeError> {
    todo!()
}
