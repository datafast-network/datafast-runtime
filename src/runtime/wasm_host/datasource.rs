use crate::info;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::asc::native_types::typed_map::AscEntity;
use crate::runtime::asc::native_types::Uint8Array;
use crate::runtime::wasm_host::Env;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn datasource_create(
    mut fenv: FunctionEnvMut<Env>,
    name_ptr: AscPtr<AscString>,
    params_ptr: AscPtr<Array<AscPtr<AscString>>>,
) -> Result<(), RuntimeError> {
    let source_name: String = asc_get(&fenv, name_ptr, 0)?;
    let source_params: Vec<String> = asc_get(&fenv, params_ptr, 0)?;
    let env = fenv.data_mut();
    env.datasource_agent
        .create_datasource(&source_name, source_params.clone(), env.block_ptr.clone())
        .unwrap();
    info!(
        wasm_host,
        "new datasource added";
        block_ptr => env.block_ptr,
        source_name => source_name,
        source_params => format!("{:?}", source_params)
    );
    Ok(())
}

pub fn datasource_create_context(
    fenv: FunctionEnvMut<Env>,
    name_ptr: AscPtr<AscString>,
    params_ptr: AscPtr<Array<AscPtr<AscString>>>,
    _context_ptr: AscPtr<AscEntity>,
) -> Result<(), RuntimeError> {
    todo!()
}

pub fn datasource_address(
    mut fenv: FunctionEnvMut<Env>,
) -> Result<AscPtr<Uint8Array>, RuntimeError> {
    let address = fenv
        .data()
        .datasource_address
        .clone()
        .map(|a| a.as_bytes().to_vec())
        .unwrap_or(vec![]);
    let address_ptr = asc_new(&mut fenv, address.as_slice())?;
    Ok(address_ptr)
}

pub fn datasource_network(
    mut fenv: FunctionEnvMut<Env>,
) -> Result<AscPtr<AscString>, RuntimeError> {
    let network = fenv.data().datasource_network.clone();
    let network_ptr = asc_new(&mut fenv, &network).unwrap();
    Ok(network_ptr)
}

pub fn datasource_context(_fenv: FunctionEnvMut<Env>) -> Result<AscPtr<AscEntity>, RuntimeError> {
    todo!()
}
