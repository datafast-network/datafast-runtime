use super::base::EntityType;
use super::base::FieldKind;
use super::base::FieldName;
use super::base::Schema;
use super::base::SchemaConfig;
use crate::common::ModeSchema;
use crate::runtime::asc::native_types::store::StoreValueKind;
use apollo_parser::cst::Argument;
use apollo_parser::cst::CstNode;
use apollo_parser::cst::Definition;
use apollo_parser::cst::Directive;
use apollo_parser::cst::Type;
use apollo_parser::Parser;
use df_logger::error;
use regex::Regex;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone, Default, Debug)]
pub struct Schemas(HashMap<EntityType, (Schema, Option<SchemaConfig>)>);

impl Schemas {
    pub fn new_from_graphql_schema(schema: &str) -> Self {
        let parser = Parser::new(schema);
        let ast = parser.parse();
        let doc = ast.document();

        let mut schemas = Schemas::default();
        doc.definitions().for_each(|def| {
            if let Definition::ObjectTypeDefinition(object) = def {
                let entity_type = object
                    .name()
                    .expect("Name of Object Definition invalid")
                    .text()
                    .to_string();
                schemas.0.insert(entity_type, (Schema::new(), None));
            }
        });
        for def in doc.definitions() {
            if let Definition::ObjectTypeDefinition(object) = def {
                let entity_type = object
                    .name()
                    .unwrap_or_else(|| panic!("Name of Object Definition invalid"))
                    .text()
                    .to_string();
                let mut schema = Schema::new();
                for field in object.fields_definition().unwrap().field_definitions() {
                    let ty = field
                        .ty()
                        .unwrap_or_else(|| panic!("Type of field {:?} error", field));
                    let field_name = field
                        .name()
                        .unwrap_or_else(|| panic!("Name of field {:?} error", field))
                        .text();
                    let mut field_kind = Self::parse_entity_field(ty);
                    if let Some(dir) = field.directives() {
                        if let Some(first) = dir.directives().next() {
                            if let Some(arg) = Self::get_args(&first) {
                                if field_kind.relation.is_some()
                                    && Self::get_name_arg(arg.clone(), "field")
                                {
                                    field_kind.relation = Some((
                                        field_kind.relation.unwrap().0,
                                        arg.value().unwrap().source_string().replace('"', ""),
                                    ));
                                }
                            }
                        }
                    }
                    schema.insert(field_name.to_string(), field_kind);
                }
                schemas.0.remove(&entity_type);
                //Get schema config
                let mut config_schema = None;
                if object.directives().is_some() {
                    let dir = object.directives().unwrap().source_string();
                    config_schema = Self::get_schema_config(&dir);
                }
                schemas.add_schema(&entity_type, schema, config_schema)
            }
        }

        schemas
    }

    fn get_schema_config(dir_str: &str) -> Option<SchemaConfig> {
        let re = Regex::new(r"@entity\(([^)]+)\)").unwrap();
        let caps = re.captures(dir_str);
        if caps.is_none() {
            return None;
        }
        let captures = caps.unwrap();
        let mut schema_config = SchemaConfig::default();
        let inner_text = captures.get(1).unwrap().as_str();
        let kv_re = Regex::new(r"(\w+):([^,]+)").unwrap();
        for capture in kv_re.captures_iter(inner_text) {
            let key = capture.get(1).unwrap().as_str();
            let value = capture.get(2).unwrap().as_str();
            match key {
                // mode readonly or write to table
                "mode" => schema_config.mode = ModeSchema::from_str(value).unwrap(),
                // namespace of database to access
                "namespace" => schema_config.namespace = Some(value.to_string()),
                //interval time (day) to truncate data 0 is never (write mode only)
                "interval" => {
                    schema_config.interval = Some(
                        value
                            .parse()
                            .expect("parse interval value from schema error"),
                    )
                }
                _ => {} // Ignore other keys
            }
        }
        Some(schema_config)
    }
    fn get_args(dir: &Directive) -> Option<Argument> {
        match dir.arguments() {
            Some(args) => args.arguments().next(),
            None => None,
        }
    }

    fn get_name_arg(argument: Argument, match_field: &str) -> bool {
        if let Some(name) = argument.name() {
            return name.text().as_str() == match_field;
        }
        false
    }

    pub fn add_schema(
        &mut self,
        entity_name: &str,
        mut schema: Schema,
        config: Option<SchemaConfig>,
    ) {
        if !schema.contains_key("__is_deleted__") {
            schema.insert(
                "__is_deleted__".to_string(),
                FieldKind {
                    kind: StoreValueKind::Bool,
                    relation: None,
                    list_inner_kind: None,
                },
            );
        }
        if !schema.contains_key("__block_ptr__") {
            schema.insert(
                "__block_ptr__".to_string(),
                FieldKind {
                    kind: StoreValueKind::Int8,
                    relation: None,
                    list_inner_kind: None,
                },
            );
        }
        self.0.insert(entity_name.to_owned(), (schema, config));
    }

    pub fn get_relation_field(
        &self,
        entity_name: &str,
        field_name: &str,
    ) -> Option<(EntityType, FieldName)> {
        let entity = self.0.get(entity_name);
        entity?;
        let field = entity.unwrap().0.get(field_name);

        field?;

        let field = field.unwrap();

        field.relation.as_ref()?;
        let relation = field.relation.clone().unwrap();
        Some(relation)
    }

    pub fn get_entity_names(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    pub fn get_schema(&self, entity_type: &str) -> Schema {
        self.0.get(entity_type).unwrap().0.clone()
    }

    pub fn get_config(&self, entity_type: &str) -> Option<SchemaConfig> {
        self.0.get(entity_type).unwrap().1.clone()
    }

    pub fn get_field(&self, entity_type: &str, field_name: &str) -> FieldKind {
        let entity_schema = self
            .0
            .get(entity_type)
            .cloned()
            .unwrap_or_else(|| panic!("No entity named = {entity_type}"));

        let field_kind = entity_schema
            .0
            .get(&field_name.replace('\"', ""))
            .cloned()
            .unwrap_or_else(|| {
                error!(Schemas, "get field name failed";
                    field_name => field_name,
                    entity_type => entity_type,
                    schema => format!("{:?}",entity_schema)
                );
                panic!("No field name = {field_name}")
            });

        field_kind
    }

    fn parse_entity_field(field_type: Type) -> FieldKind {
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
                        relation = Some((unknown_type.to_string(), "id".to_string()));
                        StoreValueKind::String
                    }
                };
                FieldKind {
                    kind,
                    relation,
                    list_inner_kind: None,
                }
            }
            Type::ListType(list) => {
                let inner_type = list.ty().expect("list type must not be None");
                let value = Schemas::parse_entity_field(inner_type);

                FieldKind {
                    kind: StoreValueKind::Array,
                    relation: value.relation,
                    list_inner_kind: Some(value.kind),
                }
            }
            Type::NonNullType(value) => {
                if let Some(list) = value.list_type() {
                    return Schemas::parse_entity_field(Type::ListType(list));
                }

                if let Some(name_type) = value.named_type() {
                    return Schemas::parse_entity_field(Type::NamedType(name_type));
                }
                unimplemented!()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use df_logger::loggers::init_logger;
    use std::fs::read_to_string;

    #[tokio::test]
    async fn test_parse_graphql_schema() {
        init_logger();

        let gql =
            read_to_string("../subgraph-testing/packages/v0_0_5/build/schema.graphql").unwrap();

        let schemas = Schemas::new_from_graphql_schema(&gql);
        let entity_type = "Pool";
        let _token = schemas.0.get(entity_type).unwrap();
    }
}
