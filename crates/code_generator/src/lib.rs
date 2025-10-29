use anyhow::Context as AnyhowContext;
use heck::ToUpperCamelCase;
use itertools::Itertools;
use lazy_static::lazy_static;
use numerics::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::{Rc, Weak},
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct TypePath(pub String);

pub struct DefinitionsTree {
    pub name: String,
    pub parent: Option<Weak<RefCell<DefinitionsTree>>>,
    pub children: std::collections::HashMap<String, RefCell<DefinitionsTree>>,
}

#[derive(Debug)]
pub struct Context {
    pub types: std::collections::HashMap<TypePath, String>,
    pub extra_types: RefCell<std::collections::HashMap<String, String>>,
    pub scope: RefCell<codegen::Scope>,
    // probably not the best way, but it makes sense
    pub extra_name: RefCell<VecDeque<String>>,
    pub strip_prefix: Option<String>,
}

pub trait ToRustTypeName {
    fn to_rust_type_name(&self, context: Rc<Context>) -> anyhow::Result<String>;
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

//TODO: probably change to an enum
#[derive(Serialize, Deserialize, Debug)]
pub struct PathItem {
    #[serde(flatten)]
    pub methods: std::collections::BTreeMap<Method, Operation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    Int32,
    Int64,
    Float,
    Double,
}

impl ToRustTypeName for NumberFormat {
    fn to_rust_type_name(&self, _context: Rc<Context>) -> anyhow::Result<String> {
        Ok(match self {
            NumberFormat::Int32 => "i32".to_string(),
            NumberFormat::Int64 => "i64".to_string(),
            NumberFormat::Float => "f32".to_string(),
            NumberFormat::Double => "f64".to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    fn to_rust_type_name(&self, context: Rc<Context>) -> anyhow::Result<String> {
        match self {
            TypeTagged::Number { format, r#enum } => {
                let format = format
                    .as_ref()
                    .map(|x| x.to_rust_type_name(context.clone()))
                    .unwrap_or(Ok("f64".to_string()))?;
                if let Some(en) = r#enum {
                    let enum_name = context
                        .extra_name
                        .borrow_mut()
                        .pop_front()
                        .context("Expected extra name for enum")?
                        .to_upper_camel_case();
                    if let Some(enuma) = context.extra_types.borrow().get(&enum_name) {
                        return Ok(enuma.to_string());
                    } else {
                        let scope = &mut context.scope.borrow_mut();
                        let enm = scope.new_enum(&enum_name);
                        enm.derive("Debug");
                        enm.derive("Serialize");
                        enm.derive("Deserialize");
                        enm.repr(&format);
                        // FIXME: make enums not crazy
                        for variant in en {
                            enm.new_variant(format!(
                                "{} = {}",
                                num_to_words::integer_to_en_us(variant.floor().to_i64().unwrap())?
                                    .to_upper_camel_case(),
                                variant
                            ));
                        }
                        context
                            .extra_types
                            .borrow_mut()
                            .insert(enum_name.clone(), enum_name.clone());
                        Ok(enum_name)
                    }
                } else {
                    Ok(format)
                }
            }
            // TODO: Handle enums properly
            TypeTagged::Integer { format, r#enum } => {
                let format = format
                    .as_ref()
                    .map(|x| x.to_rust_type_name(context.clone()))
                    .unwrap_or(Ok("i64".to_string()))?;
                if let Some(en) = r#enum {
                    let enum_name = context
                        .extra_name
                        .borrow_mut()
                        .pop_front()
                        .context("Expected extra name for enum")?
                        .to_upper_camel_case();
                    if let Some(enuma) = context.extra_types.borrow().get(&enum_name) {
                        return Ok(enuma.clone());
                    } else {
                        let scope = &mut context.scope.borrow_mut();
                        let enm = scope.new_enum(&enum_name);
                        enm.derive("Debug");
                        enm.derive("Serialize");
                        enm.derive("Deserialize");
                        enm.repr(&format);

                        for variant in en {
                            enm.new_variant(format!(
                                "{} = {}",
                                num_to_words::integer_to_en_us(*variant)?.to_upper_camel_case(),
                                variant
                            ));
                        }
                        context
                            .extra_types
                            .borrow_mut()
                            .insert(enum_name.clone(), enum_name.clone());
                        Ok(enum_name)
                    }
                } else {
                    Ok(format)
                }
            }
            // TODO: Handle enums properly
            TypeTagged::String { .. } => Ok("String".to_string()),
            TypeTagged::Boolean => Ok("bool".to_string()),
            TypeTagged::Array { items } => Ok(format!(
                "Vec<{}>",
                items.schema_object.to_rust_type_name(context)?
            )),
            // TODO: Implement proper object handling
            TypeTagged::Object { .. } => Ok("serde_json::Value".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    fn to_rust_type_name(&self, context: Rc<Context>) -> anyhow::Result<String> {
        match self {
            TypeUntagged::Tagged(tagged) => tagged.to_rust_type_name(context),
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

mod locations {
    use std::fmt::Display;

    use super::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Query;
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Header;
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Path;

    impl Display for Query {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "query")
        }
    }

    impl Display for Header {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "header")
        }
    }

    impl Display for Path {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "path")
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy)]
    #[serde(rename_all = "lowercase")]
    pub enum InLocation {
        Query,
        Header,
        Path,
        Cookie,
    }

    impl AsInLocation for InLocation {
        fn from_enum(_loc: &InLocation) -> Option<Self> {
            Some(*_loc)
        }
        fn to_enum(&self) -> InLocation {
            *self
        }
    }
    pub trait AsInLocation: Serialize + for<'de> Deserialize<'de> {
        fn from_enum(loc: &InLocation) -> Option<Self>;
        fn to_enum(&self) -> InLocation;
    }

    impl AsInLocation for Query {
        fn from_enum(loc: &InLocation) -> Option<Self> {
            match loc {
                InLocation::Query => Some(Query),
                _ => None,
            }
        }
        fn to_enum(&self) -> InLocation {
            InLocation::Query
        }
    }

    impl AsInLocation for Header {
        fn from_enum(loc: &InLocation) -> Option<Self> {
            match loc {
                InLocation::Header => Some(Header),
                _ => None,
            }
        }
        fn to_enum(&self) -> InLocation {
            InLocation::Header
        }
    }

    impl AsInLocation for Path {
        fn from_enum(loc: &InLocation) -> Option<Self> {
            match loc {
                InLocation::Path => Some(Path),
                _ => None,
            }
        }
        fn to_enum(&self) -> InLocation {
            InLocation::Path
        }
    }
}

pub use locations::{AsInLocation, InLocation};

use crate::locations::Query;

#[derive(Serialize, Deserialize, Debug)]
pub struct Parameter<T>
where
    T: AsInLocation,
{
    pub name: String,
    #[serde(rename = "in")]
    #[serde(bound = "T: AsInLocation + std::fmt::Debug")]
    pub in_: T,
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

#[derive(Debug)]
pub struct ParameterLocations {
    pub query: Vec<Parameter<locations::Query>>,
    pub header: Vec<Parameter<locations::Header>>,
    pub path: Vec<Parameter<locations::Path>>,
}

impl Default for ParameterLocations {
    fn default() -> Self {
        ParameterLocations {
            query: Vec::new(),
            header: Vec::new(),
            path: Vec::new(),
        }
    }
}

impl<'de> Deserialize<'de> for ParameterLocations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let params: Vec<Parameter<InLocation>> = Deserialize::deserialize(deserializer)?;
        let query = params
            .iter()
            .filter_map(|x| {
                if let Some(q) = locations::Query::from_enum(&x.in_) {
                    Some(Parameter {
                        name: x.name.clone(),
                        in_: q,
                        required: x.required,
                        r#type: Type {
                            description: x.r#type.description.clone(),
                            schema_object: x.r#type.schema_object.clone(),
                        },
                        description: x.description.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let header = params
            .iter()
            .filter_map(|x| {
                if let Some(h) = locations::Header::from_enum(&x.in_) {
                    Some(Parameter {
                        name: x.name.clone(),
                        in_: h,
                        required: x.required,
                        r#type: Type {
                            description: x.r#type.description.clone(),
                            schema_object: x.r#type.schema_object.clone(),
                        },
                        description: x.description.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let path = params
            .iter()
            .filter_map(|x| {
                if let Some(p) = locations::Path::from_enum(&x.in_) {
                    Some(Parameter {
                        name: x.name.clone(),
                        in_: p,
                        required: x.required,
                        r#type: Type {
                            description: x.r#type.description.clone(),
                            schema_object: x.r#type.schema_object.clone(),
                        },
                        description: x.description.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(ParameterLocations {
            query,
            header,
            path,
        })
    }
}

impl Serialize for ParameterLocations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut params = Vec::new();
        for param in &self.query {
            params.push(serde_json::to_value(param).map_err(serde::ser::Error::custom)?);
        }
        for param in &self.header {
            params.push(serde_json::to_value(param).map_err(serde::ser::Error::custom)?);
        }
        for param in &self.path {
            params.push(serde_json::to_value(param).map_err(serde::ser::Error::custom)?);
        }
        params.serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Operation {
    pub tags: Option<Vec<String>>,
    pub operation_id: Option<String>,
    pub consumes: Option<Vec<String>>,
    pub produces: Option<Vec<String>>,
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: ParameterLocations,
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, strum::Display)]
pub enum PathElement {
    #[strum(to_string = "{0}")]
    Static(String),
    Parameter(String),
}

impl AsRef<str> for PathElement {
    fn as_ref(&self) -> &str {
        match self {
            PathElement::Static(s) => s.as_ref(),
            PathElement::Parameter(s) => s.as_ref(),
        }
    }
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

#[derive(Debug, Clone)]
pub struct PathName {
    internal: String,
    pub elements: std::collections::VecDeque<PathElement>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
