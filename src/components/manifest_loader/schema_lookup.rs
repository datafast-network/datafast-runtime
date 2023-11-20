use crate::error;
use crate::errors::ManifestLoaderError;
use crate::messages::EntityType;
use crate::messages::RawEntity;
use crate::runtime::asc::native_types::store::Bytes;
use crate::runtime::asc::native_types::store::StoreValueKind;
use crate::runtime::asc::native_types::store::Value;
use crate::runtime::bignumber::bigdecimal::BigDecimal;
use crate::runtime::bignumber::bigint::BigInt;
use apollo_parser::cst::CstNode;
use apollo_parser::cst::Definition;
use apollo_parser::cst::Type;
use apollo_parser::Parser;
use std::collections::HashMap;
use std::str::FromStr;

type FieldName = String;

#[derive(Clone, Default, Debug)]
pub struct FieldKind {
    pub kind: StoreValueKind,
    pub relation: Option<(EntityType, FieldName)>,
    pub list_inner_kind: Option<StoreValueKind>,
}

#[derive(Clone, Default)]
pub struct SchemaLookup {
    schema: HashMap<EntityType, HashMap<FieldName, FieldKind>>,
}

impl SchemaLookup {
    pub fn new() -> Self {
        Self {
            schema: HashMap::new(),
        }
    }

    pub fn new_from_graphql_schema(schema: &str) -> Result<Self, ManifestLoaderError> {
        let parser = Parser::new(schema);
        let ast = parser.parse();
        let doc = ast.document();

        let mut schema_lookup = SchemaLookup::new();
        doc.definitions().for_each(|def| {
            if let Definition::ObjectTypeDefinition(object) = def {
                let entity_type = object
                    .name()
                    .expect("Name of Object Definition invalid")
                    .text()
                    .to_string();
                schema_lookup.schema.insert(entity_type, HashMap::new());
            }
        });
        for def in doc.definitions() {
            if let Definition::ObjectTypeDefinition(object) = def {
                let entity_type = object
                    .name()
                    .unwrap_or_else(|| panic!("Name of Object Definition invalid"))
                    .text()
                    .to_string();
                let mut schema = HashMap::new();
                for field in object.fields_definition().unwrap().field_definitions() {
                    let ty = field
                        .ty()
                        .unwrap_or_else(|| panic!("Type of field {:?} error", field));
                    let field_name = field
                        .name()
                        .unwrap_or_else(|| panic!("Name of field {:?} error", field))
                        .text();
                    let mut field_kind = schema_lookup.parse_entity_field(ty)?;
                    if let Some(dir) = field.directives() {
                        let first = dir.directives().next();
                        if first.is_some() {
                            let first = first.unwrap();
                            let arg = first.arguments().unwrap().arguments().next().unwrap();
                            let name = arg.name().unwrap().text();
                            if field_kind.relation.is_some() && name == "field" {
                                field_kind.relation = Some((
                                    field_kind.relation.unwrap().0,
                                    arg.value().unwrap().source_string().replace('"', ""),
                                ));
                            }
                        }
                    }
                    schema.insert(field_name.to_string(), field_kind);
                }
                schema_lookup.schema.remove(&entity_type);
                schema_lookup.add_schema(&entity_type, schema)
            }
        }

        Ok(schema_lookup)
    }

    pub fn add_schema(&mut self, entity_name: &str, schema: HashMap<String, FieldKind>) {
        self.schema.insert(entity_name.to_owned(), schema);
    }

    pub fn get_relation_field(
        &self,
        entity_name: &str,
        field_name: &str,
    ) -> Option<(EntityType, FieldName)> {
        let entity = self.schema.get(entity_name);
        entity?;
        let field = entity.unwrap().get(field_name);

        field?;

        let field = field.unwrap();

        field.relation.as_ref()?;
        let relation = field.relation.clone().unwrap();
        Some(relation)
    }

    pub fn get_schemas(&self) -> &HashMap<String, HashMap<String, FieldKind>> {
        &self.schema
    }

    pub fn get_entity_names(&self) -> Vec<String> {
        self.schema.keys().cloned().collect()
    }

    fn get_field(&self, entity_type: &str, field_name: &str) -> FieldKind {
        if field_name == "block_ptr_number" {
            return FieldKind {
                kind: StoreValueKind::Int8,
                relation: None,
                list_inner_kind: None,
            };
        }
        if field_name == "is_deleted" {
            return FieldKind {
                kind: StoreValueKind::Bool,
                relation: None,
                list_inner_kind: None,
            };
        }

        self.schema
            .get(entity_type)
            .expect("get entity type error")
            .get(field_name.replace('"', "").as_str())
            .expect("get field error")
            .clone()
    }

    pub fn json_to_entity(
        &self,
        entity_name: &str,
        json: serde_json::Map<String, serde_json::Value>,
    ) -> RawEntity {
        let mut result = HashMap::new();

        for (key, val) in json {
            let field_type = self.get_field(entity_name, &key);
            let value = self.field_to_store_value(field_type, val);
            result.insert(key.replace('"', ""), value);
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
            let value = self.store_value_to_json_value(value);
            result.insert(key.replace('"', ""), value);
        }

        result
    }

    fn parse_entity_field(&mut self, field_type: Type) -> Result<FieldKind, ManifestLoaderError> {
        match field_type {
            Type::NamedType(name_type) => {
                let type_name = name_type
                    .name()
                    .unwrap_or_else(|| panic!("get type name for field {:?} error", name_type))
                    .text()
                    .to_owned();
                let mut relation = None;
                let kind = match type_name.as_str() {
                    "ID" => StoreValueKind::String,
                    "BigInt" => StoreValueKind::BigInt,
                    "BigDecimal" => StoreValueKind::BigDecimal,
                    "Bytes" => StoreValueKind::Bytes,
                    "String" => StoreValueKind::String,
                    "Boolean" => StoreValueKind::Bool,
                    "Int" => StoreValueKind::Int,
                    "Int8" => StoreValueKind::Int8,
                    unknown_type => {
                        if self.schema.get(&type_name).is_some() {
                            relation = Some((unknown_type.to_string(), "id".to_string()));
                            StoreValueKind::Bytes
                        } else {
                            error!(parse_entity_field, "Unknown schema type";
                                field_type => unknown_type,
                                type_name => type_name
                            );
                            return Err(ManifestLoaderError::SchemaParsingError);
                        }
                    }
                };
                Ok(FieldKind {
                    kind,
                    relation,
                    list_inner_kind: None,
                })
            }
            Type::ListType(list) => {
                let inner_type = list.ty();
                if inner_type.is_none() {
                    error!(parse_entity_field, "List type is None";
                        field_type => format!("{:?}", list)
                    );
                    return Err(ManifestLoaderError::SchemaParsingError);
                }
                let value = self.parse_entity_field(inner_type.unwrap())?;
                let array_kind = FieldKind {
                    kind: StoreValueKind::Array,
                    relation: value.relation,
                    list_inner_kind: Some(value.kind),
                };
                Ok(array_kind)
            }
            Type::NonNullType(value) => {
                if let Some(list) = value.list_type() {
                    return self.parse_entity_field(Type::ListType(list));
                }

                if let Some(name_type) = value.named_type() {
                    return self.parse_entity_field(Type::NamedType(name_type));
                }
                unimplemented!()
            }
        }
    }

    fn field_to_store_value(&self, field_kind: FieldKind, val: serde_json::Value) -> Value {
        match field_kind.kind {
            StoreValueKind::String => Value::String(val.to_string()),
            StoreValueKind::Int => Value::Int(val.as_i64().unwrap() as i32),
            StoreValueKind::Int8 => Value::Int8(val.as_i64().unwrap()),
            StoreValueKind::BigDecimal => {
                Value::BigDecimal(BigDecimal::from_str(&val.to_string()).unwrap())
            }
            StoreValueKind::Bool => Value::Bool(val.as_bool().unwrap()),
            StoreValueKind::Bytes => {
                let hex_bytes = hex::decode(val.as_str().unwrap()).unwrap();
                Value::Bytes(Bytes::from(hex_bytes))
            }
            StoreValueKind::BigInt => Value::BigInt(BigInt::from_str(&val.to_string()).unwrap()),
            StoreValueKind::Array => {
                let mut result = Vec::new();
                let inner_kind = field_kind.list_inner_kind.unwrap();
                if val.is_null() {
                    error!(field_to_store_value, "get array error";
                        val => format!("{:?}", val),
                        field_kind => format!("{:?}", field_kind)
                    );
                    return Value::List(result);
                }
                for item in val.as_array().expect("get array error").iter() {
                    let field_kind_array = FieldKind {
                        kind: inner_kind,
                        relation: None,
                        list_inner_kind: None,
                    };
                    let item = self.field_to_store_value(field_kind_array, item.clone());
                    result.push(item);
                }
                Value::List(result)
            }
            StoreValueKind::Null => Value::Null,
        }
    }

    fn store_value_to_json_value(&self, value: Value) -> serde_json::Value {
        match value {
            Value::Int(number) => serde_json::Value::from(number),
            Value::Int8(number) => serde_json::Value::from(number),
            Value::String(string) => serde_json::Value::String(string),
            Value::BigDecimal(number) => serde_json::Value::from(number.to_string()),
            Value::BigInt(number) => serde_json::Value::from(number.to_string()),
            Value::Bytes(bytes) => serde_json::Value::from(hex::encode(bytes.as_slice())),
            Value::Bool(bool_val) => serde_json::Value::Bool(bool_val),
            Value::List(list) => {
                let mut result = Vec::new();
                for item in list {
                    let item = self.store_value_to_json_value(item);
                    result.push(item);
                }
                serde_json::Value::Array(result)
            }
            Value::Null => serde_json::Value::Null,
        }
    }

    fn get_field_kind_to_scyladb_type(field_kind: FieldKind) -> String {
        match field_kind.kind {
            StoreValueKind::String => "text".to_string(),
            StoreValueKind::Int => "int".to_string(),
            StoreValueKind::Int8 => "bigint".to_string(),
            StoreValueKind::BigDecimal => "decimal".to_string(),
            StoreValueKind::Bool => "boolean".to_string(),
            StoreValueKind::Bytes => "blob".to_string(),
            StoreValueKind::BigInt => "bigint".to_string(),
            StoreValueKind::Array => {
                let inner_kind = field_kind.list_inner_kind.unwrap();
                let inner_kind = FieldKind {
                    kind: inner_kind,
                    relation: None,
                    list_inner_kind: None,
                };
                let inner_kind_name = Self::get_field_kind_to_scyladb_type(inner_kind);
                format!("list<{}>", inner_kind_name)
            }
            StoreValueKind::Null => unimplemented!(),
        }
    }

    pub fn generate_insert_query(
        &self,
        keyspace: &str,
        entity_name: &str,
        values: serde_json::Map<String, serde_json::Value>,
    ) -> String {
        let mut query = String::new();
        query.push_str(&format!("INSERT INTO {keyspace}.\"{entity_name}\""));
        query.push('(');
        let mut fields = Vec::new();
        for (field_name, _) in self.schema.get(entity_name).unwrap().iter() {
            let field_kind = self.get_field(entity_name, field_name);
            let field_type = Self::get_field_kind_to_scyladb_type(field_kind);
            fields.push(format!("{} {}", field_name, field_type));
        }
        query.push_str(fields.join(", ").as_str());
        query.push_str(") VALUES (");
        let mut values_inserts = Vec::new();
        for (field_name, _) in self.schema.get(entity_name).unwrap().iter() {
            let value = values.get(field_name).unwrap();
            values_inserts.push(value.clone());
        }
        query
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use log::info;
    use std::fs::read_to_string;

    #[tokio::test]
    async fn test_parse_graphql_schema() {
        env_logger::try_init().unwrap_or_default();

        let gql =
            read_to_string("../subgraph-testing/packages/v0_0_5/build/schema.graphql").unwrap();

        let schema_lookup = SchemaLookup::new_from_graphql_schema(&gql).unwrap();
        let entity_type = "Pool";
        let token = schema_lookup.schema.get(entity_type).unwrap();
        info!("Token: {:?}", token);
    }

    #[test]
    fn test_parse_array() {
        env_logger::try_init().unwrap_or_default();
        let field_kind = FieldKind {
            kind: StoreValueKind::Array,
            relation: None,
            list_inner_kind: Some(StoreValueKind::String),
        };
        let val = serde_json::Value::Array(vec![
            serde_json::Value::String("a".to_string()),
            serde_json::Value::String("b".to_string()),
            serde_json::Value::String("c".to_string()),
        ]);
        let schema = SchemaLookup::new();
        let result = schema.field_to_store_value(field_kind, val);

        assert_eq!(
            result,
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ])
        );
        //case string is bytes
        let field_kind = FieldKind {
            kind: StoreValueKind::Array,
            relation: None,
            list_inner_kind: Some(StoreValueKind::Bytes),
        };
        let val = serde_json::Value::Array(vec![
            serde_json::Value::String("0x8A9d69Aa686fA0f9BbDec21294F67D4D9CFb4A3E".to_string()),
            serde_json::Value::String("0xd69B8fF1888e78d9C337C2f2e6b3Bf3E7357800E".to_string()),
        ]);

        let result = schema.field_to_store_value(field_kind, val);
        assert_eq!(
            result,
            Value::List(vec![
                Value::Bytes(Bytes::from(
                    "0x8A9d69Aa686fA0f9BbDec21294F67D4D9CFb4A3E".as_bytes()
                )),
                Value::Bytes(Bytes::from(
                    "0xd69B8fF1888e78d9C337C2f2e6b3Bf3E7357800E".as_bytes()
                )),
            ])
        );
        //case string is bigint
        let field_kind = FieldKind {
            kind: StoreValueKind::Array,
            relation: None,
            list_inner_kind: Some(StoreValueKind::BigInt),
        };
        let val = serde_json::Value::Array(vec![
            serde_json::Value::String("1234567890123456789012345678901234567890".to_string()),
            serde_json::Value::String("1234567890123456789012345678901234567890".to_string()),
        ]);

        let result = schema.field_to_store_value(field_kind, val);

        assert_eq!(
            result,
            Value::List(vec![
                Value::BigInt(
                    BigInt::from_str("1234567890123456789012345678901234567890").unwrap()
                ),
                Value::BigInt(
                    BigInt::from_str("1234567890123456789012345678901234567890").unwrap()
                ),
            ])
        );

        //case string is bigdecimal
        let field_kind = FieldKind {
            kind: StoreValueKind::Array,
            relation: None,
            list_inner_kind: Some(StoreValueKind::BigDecimal),
        };
        let val = serde_json::Value::Array(vec![
            serde_json::Value::String(
                "1234567890123456789012345678901234567890.1234567890123456789012345678901234567890"
                    .to_string(),
            ),
            serde_json::Value::String(
                "1234567890123456789012345678901234567890.1234567890123456789012345678901234567890"
                    .to_string(),
            ),
        ]);

        let result = schema.field_to_store_value(field_kind, val);

        assert_eq!(
            result,
            Value::List(vec![
                Value::BigDecimal(
                    BigDecimal::from_str("1234567890123456789012345678901234567890.1234567890123456789012345678901234567890").unwrap()
                ),
                Value::BigDecimal(
                    BigDecimal::from_str("1234567890123456789012345678901234567890.1234567890123456789012345678901234567890").unwrap()
                ),
            ])
        );
    }
}
