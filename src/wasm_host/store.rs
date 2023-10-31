use super::Env;
use crate::asc::base::asc_get;
use crate::asc::base::AscPtr;
use crate::asc::native_types::string::AscString;
use crate::asc::native_types::typed_map::AscEntity;
use crate::internal_messages::StoreOperationMessage;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn store_set(
    mut fenv: FunctionEnvMut<Env>,
    entity_ptr: AscPtr<AscString>,
    id_ptr: AscPtr<AscString>,
    data_ptr: AscPtr<AscEntity>,
) -> Result<(), RuntimeError> {
    let (mut env, mut store) = fenv.data_and_store_mut();
    let mut db = env.db_agent.as_ref().unwrap();
    let entity: String = asc_get(&fenv, entity_ptr, 0)?;
    let id: String = asc_get(&fenv, id_ptr, 0)?;
    let data = asc_get(&fenv, data_ptr, 0)?;
    // FIXME: Update or insert new
    let request = StoreOperationMessage::Update(data);
    let result = db
        .send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;
    Ok(())
}

pub fn store_get(fenv: FunctionEnvMut<Env>, _: i32, _: i32) -> Result<(), RuntimeError> {
    Ok(())
}
