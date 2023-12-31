use super::Env;
use crate::common::RawEntity;
use crate::common::StoreOperationMessage;
use crate::common::StoreRequestResult;
use crate::runtime::asc::base::asc_get;
use crate::runtime::asc::base::asc_new;
use crate::runtime::asc::base::AscPtr;
use crate::runtime::asc::native_types::array::Array;
use crate::runtime::asc::native_types::store::Value;
use crate::runtime::asc::native_types::string::AscString;
use crate::runtime::asc::native_types::typed_map::AscEntity;
use std::collections::HashMap;
use wasmer::FunctionEnvMut;
use wasmer::RuntimeError;

pub fn store_set(
    fenv: FunctionEnvMut<Env>,
    entity_type_ptr: AscPtr<AscString>,
    entity_id_ptr: AscPtr<AscString>,
    data_ptr: AscPtr<AscEntity>,
) -> Result<(), RuntimeError> {
    let env = fenv.data();
    let db = env.db.clone();
    let entity_id: String = asc_get(&fenv, entity_id_ptr, 0)?;
    let mut data: HashMap<String, Value> = asc_get(&fenv, data_ptr, 0)?;
    let entity_type: String = asc_get(&fenv, entity_type_ptr, 0)?;

    if !data.contains_key("id") {
        // WARN: v0.0.5 Entity has `id` stripped off (why???)
        // If entity is not yet created, forcing ID might not be a good idea
        data.insert("id".to_string(), Value::String(entity_id.clone()));
    }

    let request = StoreOperationMessage::Update((entity_type, entity_id, data));
    let _result = db
        .wasm_send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;

    Ok(())
}

pub fn store_get(
    mut fenv: FunctionEnvMut<Env>,
    entity_type_ptr: AscPtr<AscString>,
    entity_id_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscEntity>, RuntimeError> {
    let entity_type: String = asc_get(&fenv, entity_type_ptr, 0)?;
    let entity_id: String = asc_get(&fenv, entity_id_ptr, 0)?;
    let env = fenv.data();
    let db = env.db.clone();
    let request = StoreOperationMessage::Load((entity_type, entity_id));
    let result = db
        .wasm_send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;

    match result {
        StoreRequestResult::Load(data) => {
            if let Some(data) = data {
                let asc_result = asc_new(&mut fenv, &data.into_iter().collect::<Vec<_>>())?;
                Ok(asc_result)
            } else {
                Ok(AscPtr::null())
            }
        }
        other => Err(RuntimeError::new(format!(
            "Load entity failed, recevied response: {:?}",
            other
        ))),
    }
}

pub fn store_remove(
    fenv: FunctionEnvMut<Env>,
    entity_type_ptr: AscPtr<AscString>,
    entity_id_ptr: AscPtr<AscString>,
) -> Result<(), RuntimeError> {
    let env = fenv.data();
    let db = env.db.clone();
    let entity_id: String = asc_get(&fenv, entity_id_ptr, 0)?;
    let entity_type: String = asc_get(&fenv, entity_type_ptr, 0)?;

    let request = StoreOperationMessage::Delete((entity_type, entity_id));
    let _result = db
        .wasm_send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;

    Ok(())
}

pub fn store_get_in_block(
    mut fenv: FunctionEnvMut<Env>,
    entity_type_ptr: AscPtr<AscString>,
    entity_id_ptr: AscPtr<AscString>,
) -> Result<AscPtr<AscEntity>, RuntimeError> {
    let entity_id: String = asc_get(&fenv, entity_id_ptr, 0)?;
    let entity_type: String = asc_get(&fenv, entity_type_ptr, 0)?;
    let db = fenv.data().db.clone();
    let request = StoreOperationMessage::LoadInBlock((entity_type, entity_id));
    let result = db
        .wasm_send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;

    match result {
        StoreRequestResult::LoadInBlock(raw_entity) => {
            if let Some(entity) = raw_entity {
                let entity = remove_private_field(vec![entity]).pop().unwrap();
                let asc_result = asc_new(&mut fenv, &entity.into_iter().collect::<Vec<_>>())?;
                Ok(asc_result)
            } else {
                Ok(AscPtr::null())
            }
        }
        _ => unimplemented!(),
    }
}

pub fn store_load_related(
    mut fenv: FunctionEnvMut<Env>,
    entity_type_ptr: AscPtr<AscString>,
    entity_id_ptr: AscPtr<AscString>,
    field_ptr: AscPtr<AscString>,
) -> Result<AscPtr<Array<AscPtr<AscEntity>>>, RuntimeError> {
    let env = fenv.data();
    let db = env.db.clone();
    let entity_id: String = asc_get(&fenv, entity_id_ptr, 0)?;
    let entity_type: String = asc_get(&fenv, entity_type_ptr, 0)?;
    let field_name: String = asc_get(&fenv, field_ptr, 0)?;

    let request = StoreOperationMessage::LoadRelated((entity_type, entity_id, field_name));
    let result = db
        .wasm_send_store_request(request)
        .map_err(|e| RuntimeError::new(e.to_string()))?;
    match result {
        StoreRequestResult::LoadRelated(entities) => {
            let entities = remove_private_field(entities);
            let vec_entities: Vec<Vec<(String, Value)>> = entities
                .into_iter()
                .map(|e| e.into_iter().collect::<Vec<_>>())
                .collect();

            let array_ptr = asc_new(&mut fenv, &vec_entities)?;

            Ok(array_ptr)
        }
        _ => unimplemented!(),
    }
}

fn remove_private_field(entities: Vec<RawEntity>) -> Vec<RawEntity> {
    entities
        .into_iter()
        .map(|mut entity| {
            entity.remove("__block_ptr__");
            entity.remove("__is_deleted__");
            entity
        })
        .collect()
}

// #[cfg(test)]
// mod test {
//     use super::super::test::*;
//     use crate::host_fn_test;
//     use crate::common::StoreOperationMessage;
//     use crate::common::StoreRequestResult;
//     use crate::runtime::asc::base::asc_get;
//     use crate::runtime::asc::base::AscPtr;
//     use crate::runtime::asc::native_types::store::Value;
//     use crate::runtime::asc::native_types::typed_map::AscEntity;
//     use crate::runtime::bignumber::bigdecimal::BigDecimal;
//     use crate::runtime::bignumber::bigint::BigInt;
//     use std::collections::HashMap;
//     use std::str::FromStr;

//     host_fn_test!("TestStore", test_store_set, host {
//         let entity_type = "Token".to_string();
//         let entity_id = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string();
//         let data = host.db.wasm_send_store_request(StoreOperationMessage::Load((entity_type.clone(), entity_id.clone()))).unwrap();

//         if let StoreRequestResult::Load(Some(entity)) = data {
//             let id = entity.get("id").unwrap().to_owned();
//             assert_eq!(id, Value::String(entity_id));
//         } else {
//             panic!("Failed")
//         }
//     });

//     host_fn_test!("TestStore", test_store_get, host, result {
//         let entity_type = "Token".to_string();
//         let entity_id = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string();
//         let mut entity_data = HashMap::new();

//         // "totalValueLockedUSD": BigDecimal(BigDecimal(0)),
//         entity_data.insert("totalValueLockedUSD".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "whitelistPools": List([]),
//         entity_data.insert("whitelistPools".to_string(), Value::List(vec![]));
//         // "poolCount": BigInt(BigInt(0)),
//         entity_data.insert("poolCount".to_string(), Value::BigInt(BigInt::from_str("0").unwrap()));
//         // "volume": BigDecimal(BigDecimal(0)),
//         entity_data.insert("volume".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "untrackedVolumeUSD": BigDecimal(BigDecimal(0)),
//         entity_data.insert("untrackedVolumeUSD".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "totalValueLockedUSDUntracked": BigDecimal(BigDecimal(0)),
//         entity_data.insert("totalValueLockedUSDUntracked".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "feesUSD": BigDecimal(BigDecimal(0)),
//         entity_data.insert("feeUSD".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "decimals": BigInt(BigInt(10)),
//         entity_data.insert("decimals".to_string(), Value::BigInt(BigInt::from_str("0").unwrap()));
//         // "txCount": BigInt(BigInt(0)),
//         entity_data.insert("txCount".to_string(), Value::BigInt(BigInt::from_str("0").unwrap()));
//         // "name": String("MyCoin"),
//         entity_data.insert("name".to_string(), Value::String("MyCoin".to_string()));
//         // "symbol": String("MYCOIN"),
//         entity_data.insert("symbol".to_string(), Value::String("MYCOIN".to_string()));
//         // "derivedETH": BigDecimal(BigDecimal(0)),
//         entity_data.insert("derivedETH".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "totalSupply": BigInt(BigInt(1000000000000)),
//         entity_data.insert("totalSupply".to_string(), Value::BigInt(BigInt::from_str("1000000000000").unwrap()));
//         // "volumeUSD": BigDecimal(BigDecimal(0)),
//         entity_data.insert("volumeUSD".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "totalValueLocked": BigDecimal(BigDecimal(0)),
//         entity_data.insert("totalValueLocked".to_string(), Value::BigDecimal(BigDecimal::from_str("0").unwrap()));
//         // "id": String("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
//         entity_data.insert("id".to_string(), Value::String(entity_id.clone()));

//         db.wasm_send_store_request(StoreOperationMessage::Update((entity_type.clone(), entity_id,entity_data))).unwrap();
//         []
//     } {
//         let asc_entity = AscPtr::<AscEntity>::new(result.first().unwrap().unwrap_i32() as u32);
//         let entity: HashMap<String, Value> = asc_get(&host, asc_entity, 0).unwrap();
//         assert_eq!(entity.len(), 17);
//         assert_eq!(*entity.get("id").unwrap(), Value::String("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()));
//         assert_eq!(*entity.get("totalSupply").unwrap(), Value::BigInt(BigInt::from_str("1000000000000").unwrap()));
//     });
// }
