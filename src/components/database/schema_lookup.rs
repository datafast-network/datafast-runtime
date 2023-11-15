use super::RawEntity;
use crate::runtime::asc::native_types::store::Bytes;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone, Default)]
pub struct SchemaLookup {
    // Load schema.graphql
    types: HashMap<String, HashMap<String, StoreValueKind>>,
}

impl SchemaLookup {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    pub fn add_schema(&mut self, entity_name: &str, schema: HashMap<String, StoreValueKind>) {
        self.types.insert(entity_name.to_owned(), schema);
    }

    fn look_up(&self, entity_name: &str, field_name: &str) -> StoreValueKind {
        return self
            .types
            .get(entity_name)
            .unwrap()
            .get(field_name)
            .cloned()
            .unwrap();
    }

    pub fn json_to_entity(
        &self,
        entity_name: &str,
        json: serde_json::Map<String, serde_json::Value>,
    ) -> RawEntity {
        let mut result = HashMap::new();

        for (key, val) in json {
            let field_type = self.look_up(entity_name, &key);
            let value = field_to_store_value(field_type, val);
            result.insert(key, value);
        }

        result
    }

    pub fn entity_to_json(
        &self,
        _entity_name: &str,
        data: RawEntity,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut result = serde_json::Map::new();

        for (key, value) in data {
            let value = store_value_to_json_value(value);
            result.insert(key, value);
        }

        result
    }
}

fn field_to_store_value(field_type: StoreValueKind, val: serde_json::Value) -> Value {
    match field_type {
        StoreValueKind::String => Value::String(val.as_str().unwrap().to_owned()),
        StoreValueKind::Int => Value::Int(val.as_i64().unwrap() as i32),
        StoreValueKind::Int8 => Value::Int8(val.as_i64().unwrap()),
        StoreValueKind::BigDecimal => {
            Value::BigDecimal(BigDecimal::from_str(val.as_str().unwrap()).unwrap())
        }
        StoreValueKind::Bool => Value::Bool(val.as_bool().unwrap()),
        StoreValueKind::Bytes => Value::Bytes(Bytes::from(val.as_str().unwrap().as_bytes())),
        StoreValueKind::BigInt => Value::BigInt(BigInt::from_str(val.as_str().unwrap()).unwrap()),
        StoreValueKind::Array => {
            unimplemented!("Not supported")
        }
        StoreValueKind::Null => {
            unimplemented!("Not supported")
        }
    }
}

fn store_value_to_json_value(value: Value) -> serde_json::Value {
    match value {
        Value::Int(number) => serde_json::Value::from(number),
        Value::Int8(number) => serde_json::Value::from(number),
        Value::String(string) => serde_json::Value::from(string),
        Value::BigDecimal(number) => serde_json::Value::from(number.to_string()),
        Value::BigInt(number) => serde_json::Value::from(number.to_string()),
        Value::Bytes(bytes) => serde_json::Value::from(format!("0x{}", bytes)),
        Value::Bool(bool_val) => serde_json::Value::Bool(bool_val),
        _ => unimplemented!(),
    }
}
