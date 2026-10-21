//! Offline GraphQL schema model with two parsers (SDL and
//! introspection JSON) and an operation builder. Everything here is
//! pure: no network, no app state. The builder emits plain operation
//! text + a JSON variables skeleton that feed straight into the
//! existing GraphQL body mode.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// How deep the selection set / input-object skeletons recurse before
/// stopping. Keeps output readable and guards self-referential types.
const MAX_DEPTH: usize = 4;

/// A type reference with its list / non-null wrappers kept, so we can
/// both render it back (`[Int!]!`) and find the innermost named type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeRef {
    Named(String),
    NonNull(Box<TypeRef>),
    List(Box<TypeRef>),
}

impl TypeRef {
    /// Innermost named type, stripped of all wrappers.
    pub fn named(&self) -> &str {
        match self {
            TypeRef::Named(n) => n,
            TypeRef::NonNull(inner) | TypeRef::List(inner) => inner.named(),
        }
    }

    /// GraphQL-syntax rendering, e.g. `[Int!]!`.
    pub fn render(&self) -> String {
        match self {
            TypeRef::Named(n) => n.clone(),
            TypeRef::NonNull(inner) => format!("{}!", inner.render()),
            TypeRef::List(inner) => format!("[{}]", inner.render()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arg {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub args: Vec<Arg>,
    pub ty: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeDef {
    Object { fields: Vec<Field> },
    Interface { fields: Vec<Field> },
    Input { fields: Vec<Arg> },
    Enum { values: Vec<String> },
    Scalar,
    Union,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schema {
    pub types: BTreeMap<String, TypeDef>,
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Query,
    Mutation,
}

impl OperationKind {
    fn keyword(self) -> &'static str {
        match self {
            OperationKind::Query => "query",
            OperationKind::Mutation => "mutation",
        }
    }
}

pub struct GeneratedOperation {
    pub query: String,
    pub variables: String,
}

impl Schema {
    /// Root type name for an operation kind, falling back to the
    /// conventional `Query` / `Mutation` names when the schema has no
    /// explicit `schema { ... }` block. `None` when no such type exists.
    fn root_type_name(&self, kind: OperationKind) -> Option<&str> {
        let explicit = match kind {
            OperationKind::Query => self.query_type.as_deref(),
            OperationKind::Mutation => self.mutation_type.as_deref(),
        };
        let fallback = match kind {
            OperationKind::Query => "Query",
            OperationKind::Mutation => "Mutation",
        };
        let name = explicit.unwrap_or(fallback);
        self.types.contains_key(name).then_some(name)
    }

    /// Browsable list of fields on the query / mutation root type.
    pub fn root_fields(&self, kind: OperationKind) -> Vec<&Field> {
        let Some(name) = self.root_type_name(kind) else {
            return vec![];
        };
        match self.types.get(name) {
            Some(TypeDef::Object { fields }) | Some(TypeDef::Interface { fields }) => {
                fields.iter().collect()
            }
            _ => vec![],
        }
    }

    fn is_composite(&self, type_name: &str) -> bool {
        matches!(
            self.types.get(type_name),
            Some(TypeDef::Object { .. }) | Some(TypeDef::Interface { .. }) | Some(TypeDef::Union)
        )
    }

    /// Build an operation for one root field: operation text plus a JSON
    /// variables skeleton. `None` when the field is not on the root type.
    pub fn build_operation(
        &self,
        kind: OperationKind,
        field_name: &str,
    ) -> Option<GeneratedOperation> {
        let field = self
            .root_fields(kind)
            .into_iter()
            .find(|f| f.name == field_name)?
            .clone();

        let op_name = pascal_case(field_name);
        let decls: Vec<String> = field
            .args
            .iter()
            .map(|a| format!("${}: {}", a.name, a.ty.render()))
            .collect();
        let call_args: Vec<String> = field
            .args
            .iter()
            .map(|a| format!("{}: ${}", a.name, a.name))
            .collect();

        let mut header = format!("{} {}", kind.keyword(), op_name);
        if !decls.is_empty() {
            header.push_str(&format!("({})", decls.join(", ")));
        }

        let mut field_line = format!("  {}", field.name);
        if !call_args.is_empty() {
            field_line.push_str(&format!("({})", call_args.join(", ")));
        }

        let inner = field.ty.named();
        let mut query = format!("{} {{\n{}", header, field_line);
        if self.is_composite(inner) {
            let mut visiting = Vec::new();
            let mut lines = self.selection_lines(inner, 4, 0, &mut visiting);
            if lines.is_empty() {
                lines.push("    __typename".to_string());
            }
            query.push_str(" {\n");
            query.push_str(&lines.join("\n"));
            query.push_str("\n  }");
        }
        query.push_str("\n}\n");

        let variables = self.build_variables(&field.args);
        Some(GeneratedOperation { query, variables })
    }

    fn selection_lines(
        &self,
        type_name: &str,
        indent: usize,
        depth: usize,
        visiting: &mut Vec<String>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        if visiting.iter().any(|v| v == type_name) {
            return out;
        }
        let fields = match self.types.get(type_name) {
            Some(TypeDef::Object { fields }) | Some(TypeDef::Interface { fields }) => fields,
            _ => return out,
        };
        visiting.push(type_name.to_string());
        let pad = " ".repeat(indent);
        for f in fields {
            let inner = f.ty.named();
            if self.is_composite(inner) {
                if depth + 1 < MAX_DEPTH {
                    let sub = self.selection_lines(inner, indent + 2, depth + 1, visiting);
                    if !sub.is_empty() {
                        out.push(format!("{}{} {{", pad, f.name));
                        out.extend(sub);
                        out.push(format!("{}}}", pad));
                    }
                }
            } else {
                out.push(format!("{}{}", pad, f.name));
            }
        }
        visiting.pop();
        out
    }

    fn build_variables(&self, args: &[Arg]) -> String {
        let mut map = Map::new();
        for a in args {
            let mut visiting = Vec::new();
            map.insert(a.name.clone(), self.placeholder(&a.ty, 0, &mut visiting));
        }
        serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
    }

    fn placeholder(&self, ty: &TypeRef, depth: usize, visiting: &mut Vec<String>) -> Value {
        match ty {
            TypeRef::NonNull(inner) => self.placeholder(inner, depth, visiting),
            TypeRef::List(inner) => Value::Array(vec![self.placeholder(inner, depth, visiting)]),
            TypeRef::Named(name) => self.named_placeholder(name, depth, visiting),
        }
    }

    fn named_placeholder(&self, name: &str, depth: usize, visiting: &mut Vec<String>) -> Value {
        match name {
            "Int" | "Float" => Value::from(0),
            "String" | "ID" => Value::from(""),
            "Boolean" => Value::from(false),
            _ => match self.types.get(name) {
                Some(TypeDef::Enum { values }) => values
                    .first()
                    .map(|v| Value::from(v.as_str()))
                    .unwrap_or(Value::Null),
                Some(TypeDef::Input { fields }) => {
                    if depth + 1 >= MAX_DEPTH || visiting.iter().any(|v| v == name) {
                        return Value::Null;
                    }
                    visiting.push(name.to_string());
                    let mut map = Map::new();
                    for f in fields {
                        map.insert(f.name.clone(), self.placeholder(&f.ty, depth + 1, visiting));
                    }
                    visiting.pop();
                    Value::Object(map)
                }
                _ => Value::Null,
            },
        }
    }
}

fn pascal_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Operation".to_string(),
    }
}

/// Parse a schema definition (SDL). Returns a human-readable error on
/// malformed input.
pub fn parse_sdl(src: &str) -> Result<Schema, String> {
    use graphql_parser::schema::{Definition, Type as GqlType, TypeDefinition};

    fn convert(t: &GqlType<'_, String>) -> TypeRef {
        match t {
            GqlType::NamedType(n) => TypeRef::Named(n.clone()),
            GqlType::ListType(inner) => TypeRef::List(Box::new(convert(inner))),
            GqlType::NonNullType(inner) => TypeRef::NonNull(Box::new(convert(inner))),
        }
    }

    let doc = graphql_parser::schema::parse_schema::<String>(src).map_err(|e| e.to_string())?;

    let mut schema = Schema::default();
    for def in &doc.definitions {
        match def {
            Definition::SchemaDefinition(sd) => {
                schema.query_type = sd.query.clone();
                schema.mutation_type = sd.mutation.clone();
            }
            Definition::TypeDefinition(td) => match td {
                TypeDefinition::Object(o) => {
                    let fields = o
                        .fields
                        .iter()
                        .map(|f| Field {
                            name: f.name.clone(),
                            args: f
                                .arguments
                                .iter()
                                .map(|a| Arg {
                                    name: a.name.clone(),
                                    ty: convert(&a.value_type),
                                })
                                .collect(),
                            ty: convert(&f.field_type),
                        })
                        .collect();
                    schema
                        .types
                        .insert(o.name.clone(), TypeDef::Object { fields });
                }
                TypeDefinition::Interface(i) => {
                    let fields = i
                        .fields
                        .iter()
                        .map(|f| Field {
                            name: f.name.clone(),
                            args: f
                                .arguments
                                .iter()
                                .map(|a| Arg {
                                    name: a.name.clone(),
                                    ty: convert(&a.value_type),
                                })
                                .collect(),
                            ty: convert(&f.field_type),
                        })
                        .collect();
                    schema
                        .types
                        .insert(i.name.clone(), TypeDef::Interface { fields });
                }
                TypeDefinition::InputObject(io) => {
                    let fields = io
                        .fields
                        .iter()
                        .map(|a| Arg {
                            name: a.name.clone(),
                            ty: convert(&a.value_type),
                        })
                        .collect();
                    schema
                        .types
                        .insert(io.name.clone(), TypeDef::Input { fields });
                }
                TypeDefinition::Enum(e) => {
                    let values = e.values.iter().map(|v| v.name.clone()).collect();
                    schema
                        .types
                        .insert(e.name.clone(), TypeDef::Enum { values });
                }
                TypeDefinition::Scalar(s) => {
                    schema.types.insert(s.name.clone(), TypeDef::Scalar);
                }
                TypeDefinition::Union(u) => {
                    schema.types.insert(u.name.clone(), TypeDef::Union);
                }
            },
            _ => {}
        }
    }
    Ok(schema)
}

/// Parse an introspection result. Accepts either the full
/// `{ "data": { "__schema": … } }` envelope or a bare `{ "__schema": … }`.
pub fn parse_introspection(src: &str) -> Result<Schema, String> {
    let root: Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    let sch = root
        .get("data")
        .and_then(|d| d.get("__schema"))
        .or_else(|| root.get("__schema"))
        .ok_or_else(|| "No `__schema` found in introspection JSON".to_string())?;

    fn convert(v: &Value) -> TypeRef {
        match v.get("kind").and_then(Value::as_str) {
            Some("NON_NULL") => TypeRef::NonNull(Box::new(convert(&v["ofType"]))),
            Some("LIST") => TypeRef::List(Box::new(convert(&v["ofType"]))),
            _ => TypeRef::Named(
                v.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown")
                    .to_string(),
            ),
        }
    }

    fn fields_of(v: &Value) -> Vec<Field> {
        v.get("fields")
            .and_then(Value::as_array)
            .map(|fs| {
                fs.iter()
                    .map(|f| Field {
                        name: f["name"].as_str().unwrap_or_default().to_string(),
                        args: f
                            .get("args")
                            .and_then(Value::as_array)
                            .map(|args| {
                                args.iter()
                                    .map(|a| Arg {
                                        name: a["name"].as_str().unwrap_or_default().to_string(),
                                        ty: convert(&a["type"]),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        ty: convert(&f["type"]),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    let query_type = sch
        .get("queryType")
        .and_then(|t| t.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let mutation_type = sch
        .get("mutationType")
        .and_then(|t| t.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut types = BTreeMap::new();
    if let Some(type_list) = sch.get("types").and_then(Value::as_array) {
        for t in type_list {
            let Some(name) = t.get("name").and_then(Value::as_str) else {
                continue;
            };
            if name.starts_with("__") {
                continue;
            }
            let def = match t.get("kind").and_then(Value::as_str) {
                Some("OBJECT") => TypeDef::Object {
                    fields: fields_of(t),
                },
                Some("INTERFACE") => TypeDef::Interface {
                    fields: fields_of(t),
                },
                Some("INPUT_OBJECT") => TypeDef::Input {
                    fields: t
                        .get("inputFields")
                        .and_then(Value::as_array)
                        .map(|fs| {
                            fs.iter()
                                .map(|f| Arg {
                                    name: f["name"].as_str().unwrap_or_default().to_string(),
                                    ty: convert(&f["type"]),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                Some("ENUM") => TypeDef::Enum {
                    values: t
                        .get("enumValues")
                        .and_then(Value::as_array)
                        .map(|vs| {
                            vs.iter()
                                .filter_map(|v| v["name"].as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                Some("UNION") => TypeDef::Union,
                _ => TypeDef::Scalar,
            };
            types.insert(name.to_string(), def);
        }
    }
    Ok(Schema {
        types,
        query_type,
        mutation_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SDL: &str = r#"
        type Query {
            user(id: ID!): User
            search(term: String!, limit: Int): [User!]
            ping: String
        }
        type Mutation {
            createUser(input: NewUser!): User
        }
        type User {
            id: ID!
            name: String!
            role: Role
            posts: [Post!]
        }
        type Post {
            id: ID!
            title: String!
        }
        input NewUser {
            name: String!
            role: Role
        }
        enum Role { ADMIN GUEST }
    "#;

    #[test]
    fn parse_sdl_reads_roots_and_fields() {
        let schema = parse_sdl(SDL).unwrap();
        assert_eq!(schema.query_type, None); // no explicit schema block
        let query_fields: Vec<_> = schema
            .root_fields(OperationKind::Query)
            .iter()
            .map(|f| f.name.clone())
            .collect();
        assert_eq!(query_fields, vec!["user", "search", "ping"]);
        let mutation_fields: Vec<_> = schema
            .root_fields(OperationKind::Mutation)
            .iter()
            .map(|f| f.name.clone())
            .collect();
        assert_eq!(mutation_fields, vec!["createUser"]);
    }

    #[test]
    fn build_operation_expands_object_selection_and_vars() {
        let schema = parse_sdl(SDL).unwrap();
        let op = schema
            .build_operation(OperationKind::Query, "user")
            .unwrap();
        assert_eq!(
            op.query,
            "query User($id: ID!) {\n  user(id: $id) {\n    id\n    name\n    role\n    posts {\n      id\n      title\n    }\n  }\n}\n"
        );
        assert_eq!(op.variables, "{\n  \"id\": \"\"\n}");
    }

    #[test]
    fn build_operation_scalar_return_has_no_selection() {
        let schema = parse_sdl(SDL).unwrap();
        let op = schema
            .build_operation(OperationKind::Query, "ping")
            .unwrap();
        assert_eq!(op.query, "query Ping {\n  ping\n}\n");
        assert_eq!(op.variables, "{}");
    }

    #[test]
    fn build_operation_list_and_input_variable_skeletons() {
        let schema = parse_sdl(SDL).unwrap();
        let search = schema
            .build_operation(OperationKind::Query, "search")
            .unwrap();
        assert!(search
            .query
            .starts_with("query Search($term: String!, $limit: Int)"));
        // Operation declares args in order; the JSON skeleton keys are
        // alphabetized (serde_json::Map is a BTreeMap).
        assert_eq!(search.variables, "{\n  \"limit\": 0,\n  \"term\": \"\"\n}");

        let create = schema
            .build_operation(OperationKind::Mutation, "createUser")
            .unwrap();
        // Enum resolves to its first value; nested input object expands.
        let vars: Value = serde_json::from_str(&create.variables).unwrap();
        assert_eq!(vars["input"]["name"], Value::from(""));
        assert_eq!(vars["input"]["role"], Value::from("ADMIN"));
    }

    #[test]
    fn introspection_matches_equivalent_sdl() {
        let introspection = r#"{
            "data": { "__schema": {
                "queryType": { "name": "Query" },
                "mutationType": null,
                "types": [
                    { "kind": "OBJECT", "name": "Query", "fields": [
                        { "name": "ping", "args": [],
                          "type": { "kind": "SCALAR", "name": "String", "ofType": null } },
                        { "name": "user",
                          "args": [ { "name": "id",
                            "type": { "kind": "NON_NULL", "name": null,
                              "ofType": { "kind": "SCALAR", "name": "ID", "ofType": null } } } ],
                          "type": { "kind": "OBJECT", "name": "User", "ofType": null } }
                    ] },
                    { "kind": "OBJECT", "name": "User", "fields": [
                        { "name": "id", "args": [],
                          "type": { "kind": "NON_NULL", "name": null,
                            "ofType": { "kind": "SCALAR", "name": "ID", "ofType": null } } }
                    ] },
                    { "kind": "SCALAR", "name": "String" },
                    { "kind": "OBJECT", "name": "__Type", "fields": [] }
                ]
            } }
        }"#;
        let schema = parse_introspection(introspection).unwrap();
        assert_eq!(schema.query_type.as_deref(), Some("Query"));
        assert_eq!(schema.mutation_type, None);
        // Introspection meta-types (`__Type`) are dropped.
        assert!(!schema.types.contains_key("__Type"));
        let op = schema
            .build_operation(OperationKind::Query, "user")
            .unwrap();
        assert_eq!(
            op.query,
            "query User($id: ID!) {\n  user(id: $id) {\n    id\n  }\n}\n"
        );
        assert_eq!(op.variables, "{\n  \"id\": \"\"\n}");
    }

    #[test]
    fn missing_schema_key_is_an_error() {
        assert!(parse_introspection("{\"data\":{}}").is_err());
        assert!(parse_sdl("type Query { = broken").is_err());
    }
}
