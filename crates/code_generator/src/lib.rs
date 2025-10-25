use anyhow::Context as AnyhowContext;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use std::cell::RefCell;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypePath(pub String);

#[derive(Debug)]
pub struct Context {
    pub types: std::collections::HashMap<TypePath, String>,
    pub extra_name: RefCell<Option<String>>,
    pub strip_prefix: Option<String>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            strip_prefix: None,
            types: std::collections::HashMap::new(),
            extra_name: RefCell::new(None),
        }
    }
}

pub trait ToRustTypeName {
    fn to_rust_type_name(
        &self,
        context: &mut Context,
        scope: &mut codegen::Scope,
    ) -> anyhow::Result<String>;
}

#[derive(Serialize, Deserialize, Debug)]
struct Info {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub terms_of_service: Option<String>,
    pub contact: Option<Contact>,
    pub license: Option<License>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contact {
    pub name: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct License {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
    pub put: Option<Operation>,
    pub delete: Option<Operation>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    Int32,
    Int64,
    Float,
    Double,
}

impl ToRustTypeName for NumberFormat {
    fn to_rust_type_name(
        &self,
        _context: &mut Context,
        _scope: &mut codegen::Scope,
    ) -> anyhow::Result<String> {
        Ok(match self {
            NumberFormat::Int32 => "i32".to_string(),
            NumberFormat::Int64 => "i64".to_string(),
            NumberFormat::Float => "f32".to_string(),
            NumberFormat::Double => "f64".to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum TypeTagged {
    Number {
        format: Option<NumberFormat>,
        r#enum: Option<Vec<f64>>,
    },
    Integer {
        format: Option<NumberFormat>,
        r#enum: Option<Vec<i64>>,
    },

    String {
        r#enum: Option<Vec<String>>,
    },
    Boolean,
    Array {
        items: Box<Type>,
    },
    Object {
        properties: Option<std::collections::HashMap<String, Type>>,
        #[serde(rename = "additionalProperties")]
        additional_properties: Option<Box<Type>>,
        required: Option<Vec<String>>,
    },
}

impl ToRustTypeName for TypeTagged {
    fn to_rust_type_name(
        &self,
        context: &mut Context,
        scope: &mut codegen::Scope,
    ) -> anyhow::Result<String> {
        match self {
            TypeTagged::Number { format, r#enum } => {
                let format = format
                    .as_ref()
                    .map(|x| x.to_rust_type_name(context, scope))
                    .unwrap_or(Ok("f64".to_string()))?;
                if let Some(en) = r#enum {
                    let enum_name = context
                        .extra_name
                        .take()
                        .context("Expected extra name for enum")?;
                    let enm = scope.new_enum(&enum_name);
                    enm.derive("Debug");
                    enm.derive("Serialize");
                    enm.derive("Deserialize");
                    enm.repr(&format);
                    for variant in en {
                        enm.new_variant(format!("{}", variant));
                    }
                    Ok(enum_name)
                } else {
                    Ok(format)
                }
            }
            TypeTagged::Integer { .. } => Ok("i64".to_string()),
            TypeTagged::String { .. } => Ok("String".to_string()),
            TypeTagged::Boolean => Ok("bool".to_string()),
            TypeTagged::Array { items } => Ok(format!(
                "Vec<{}>",
                items.schema_object.to_rust_type_name(context, scope)?
            )),
            TypeTagged::Object { .. } => Ok("serde_json::Value".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TypeUntagged {
    Tagged(TypeTagged),
    Ref {
        #[serde(rename = "$ref")]
        r#ref: TypePath,
    },
    //   Extra(serde_json::Value),
}

impl ToRustTypeName for TypeUntagged {
    fn to_rust_type_name(
        &self,
        context: &mut Context,
        scope: &mut codegen::Scope,
    ) -> anyhow::Result<String> {
        match self {
            TypeUntagged::Tagged(tagged) => tagged.to_rust_type_name(context, scope),
            TypeUntagged::Ref { r#ref } => {
                let type_name = context
                    .types
                    .get(r#ref)
                    .cloned()
                    .unwrap_or("serde_json::Value".to_string());
                Ok(type_name.clone())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InLocation {
    Query,
    Header,
    Path,
    Cookie,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub in_: InLocation,
    pub required: bool,
    #[serde(flatten)]
    pub r#type: Type,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub description: Option<String>,
    pub schema: Option<Type>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Operation {
    pub tags: Option<Vec<String>>,
    pub operation_id: Option<String>,
    pub consumes: Option<Vec<String>>,
    pub produces: Option<Vec<String>>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<Vec<Parameter>>,
    pub responses: std::collections::HashMap<String, Response>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SwaggerVersion {
    #[serde(rename = "2.0")]
    V2,
    #[serde(rename = "3.0")]
    V3,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Http,
    Https,
    Ws,
    Wss,
}

#[derive(Debug)]
enum PathElement {
    Static(String),
    Parameter(String),
}

impl<'de> Deserialize<'de> for PathName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let elements = s
            .split('/')
            .filter(|part| !part.is_empty())
            .map(|part| {
                if part.starts_with('{') && part.ends_with('}') {
                    PathElement::Parameter(part[1..part.len() - 1].to_string())
                } else {
                    PathElement::Static(part.to_string())
                }
            })
            .collect();
        Ok(PathName {
            internal: s,
            elements,
        })
    }
}

#[derive(Debug)]
pub struct PathName {
    internal: String,
    elements: Vec<PathElement>,
}

impl PartialEq for PathName {
    fn eq(&self, other: &Self) -> bool {
        self.internal == other.internal
    }
}
impl Eq for PathName {}

impl std::hash::Hash for PathName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.internal.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Type {
    pub description: Option<String>,
    #[serde(flatten)]
    pub schema_object: TypeUntagged,
}

#[derive(Deserialize, Debug)]
pub struct SwaggerFile {
    pub swagger: SwaggerVersion,
    pub info: Info,
    pub host: String,
    pub schemes: Vec<Scheme>,
    pub paths: std::collections::HashMap<PathName, PathItem>,
    pub definitions: std::collections::HashMap<String, Type>,
}
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
