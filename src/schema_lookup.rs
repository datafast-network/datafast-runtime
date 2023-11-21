use crate::error;
use crate::messages::EntityType;
use crate::runtime::asc::native_types::store::StoreValueKind;
use apollo_parser::cst::CstNode;
use apollo_parser::cst::Definition;
use apollo_parser::cst::Type;
use apollo_parser::Parser;
use std::collections::BTreeMap;
use std::collections::HashMap;

type FieldName = String;
pub type Schema = BTreeMap<FieldName, FieldKind>;

#[derive(Clone, Default, Debug)]
pub struct FieldKind {
    pub kind: StoreValueKind,
    pub relation: Option<(EntityType, FieldName)>,
    pub list_inner_kind: Option<StoreValueKind>,
}

#[derive(Clone, Default)]
pub struct SchemaLookup {
    schema: HashMap<EntityType, Schema>,
}

impl SchemaLookup {
    pub fn new() -> Self {
        Self {
            schema: HashMap::new(),
        }
    }

    pub fn new_from_graphql_schema(schema: &str) -> Self {
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
                schema_lookup.schema.insert(entity_type, Schema::new());
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
                        let first = dir.directives().next();
                        if let Some(first) = first {
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

        schema_lookup
    }

    pub fn add_schema(&mut self, entity_name: &str, schema: Schema) {
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

    pub fn get_entity_names(&self) -> Vec<String> {
        self.schema.keys().cloned().collect()
    }

    pub fn get_schema(&self, entity_type: &str) -> Schema {
        self.schema.get(entity_type).unwrap().clone()
    }

    pub fn get_field(&self, entity_type: &str, field_name: &str) -> FieldKind {
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

        let entity_schema = self
            .schema
            .get(entity_type)
            .cloned()
            .unwrap_or_else(|| panic!("No entity named = {entity_type}"));

        let field_kind = entity_schema
            .get(&field_name.replace('\"', ""))
            .cloned()
            .unwrap_or_else(|| {
                error!(SchemaLookup, "get field name failed";
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
                let value = SchemaLookup::parse_entity_field(inner_type);

                FieldKind {
                    kind: StoreValueKind::Array,
                    relation: value.relation,
                    list_inner_kind: Some(value.kind),
                }
            }
            Type::NonNullType(value) => {
                if let Some(list) = value.list_type() {
                    return SchemaLookup::parse_entity_field(Type::ListType(list));
                }

                if let Some(name_type) = value.named_type() {
                    return SchemaLookup::parse_entity_field(Type::NamedType(name_type));
                }
                unimplemented!()
            }
        }
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

        let schema_lookup = SchemaLookup::new_from_graphql_schema(&gql);
        let entity_type = "Pool";
        let token = schema_lookup.schema.get(entity_type).unwrap();
        info!("Token: {:?}", token);
    }
}
