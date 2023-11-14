use super::RawEntity;
use crate::runtime::{
    asc::native_types::{store::Bytes, store::Value},
    bignumber::{bigdecimal::BigDecimal, bigint::BigInt},
};
use std::{collections::HashMap, str::FromStr};

#[derive(Clone, Default)]
pub struct SchemaLookup {
    // Load schema.graphql
    types: HashMap<String, HashMap<String, String>>,
}

impl SchemaLookup {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    pub fn add_schema(&mut self, entity_name: String, schema: HashMap<String, String>) {
        self.types.insert(entity_name, schema);
    }

    fn look_up(&self, entity_name: &str, field_name: &str) -> String {
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
        entity_name: String,
        json: serde_json::Map<String, serde_json::Value>,
    ) -> RawEntity {
        let mut result = HashMap::new();

        for (key, val) in json {
            let field_type = self.look_up(&entity_name, &key);
            let value = field_to_store_value(&field_type, val);
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

fn field_to_store_value(field_type: &str, val: serde_json::Value) -> Value {
    match field_type {
        "String" => Value::String(val.to_string()),
        "Int" => Value::Int(val.as_i64().unwrap() as i32),
        "Int8" => Value::Int8(val.as_i64().unwrap()),
        "BigDecimal" => Value::BigDecimal(BigDecimal::from_str(val.as_str().unwrap()).unwrap()),
        "Bool" => Value::Bool(val.as_bool().unwrap()),
        "List" => {
            unimplemented!("Not supported")
        }
        "Bytes" => Value::Bytes(Bytes::from(val.as_str().clone().unwrap().as_bytes())),
        "BigInt" => Value::BigInt(BigInt::from_str(val.as_str().unwrap()).unwrap()),
        _ => todo!(),
    }
}

fn store_value_to_json_value(value: Value) -> serde_json::Value {
    match value {
        Value::Int(number) => serde_json::Value::from(number),
        Value::Int8(number) => serde_json::Value::from(number),
        Value::String(string) => serde_json::Value::from(string),
        Value::BigDecimal(number) => serde_json::Value::from(number.to_string()),
        Value::BigInt(number) => serde_json::Value::from(number.to_string()),
        Value::Bytes(bytes) => serde_json::Value::from(format!("0x{}", bytes.to_string())),
        Value::Bool(bool_val) => serde_json::Value::Bool(bool_val),
        _ => unimplemented!(),
    }
}
