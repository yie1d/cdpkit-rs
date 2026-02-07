use heck::ToSnakeCase;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Protocol {
    pub version: Version,
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    pub major: String,
    pub minor: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Domain {
    pub domain: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub types: Vec<TypeDef>,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TypeDef {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default)]
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
    #[serde(default)]
    pub items: Option<Box<TypeRef>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub parameters: Vec<Property>,
    #[serde(default)]
    pub returns: Vec<Property>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub parameters: Vec<Property>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(flatten)]
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TypeRef {
    Simple {
        #[serde(rename = "type")]
        type_: String,
        #[serde(default)]
        items: Option<Box<TypeRef>>,
    },
    Ref {
        #[serde(rename = "$ref")]
        ref_: String,
    },
}

impl TypeRef {
    /// Generate Rust type reference.
    /// `from_types_submod`: true when generating code inside the `types` submodule (same-domain refs are plain names),
    /// false when generating inside `methods`/`events` submodules (same-domain refs need `types::` prefix).
    pub fn to_rust_type(&self, domain: &str, all_types: &HashMap<String, String>, from_types_submod: bool) -> String {
        match self {
            TypeRef::Simple { type_, items } => match type_.as_str() {
                "string" => "String".to_string(),
                "integer" => "i64".to_string(),
                "number" => "f64".to_string(),
                "boolean" => "bool".to_string(),
                "array" => {
                    if let Some(items) = items {
                        format!("Vec<{}>", items.to_rust_type(domain, all_types, from_types_submod))
                    } else {
                        "Vec<serde_json::Value>".to_string()
                    }
                }
                "object" => "serde_json::Value".to_string(),
                "any" => "serde_json::Value".to_string(),
                _ => "serde_json::Value".to_string(),
            },
            TypeRef::Ref { ref_ } => {
                if ref_.contains('.') {
                    // Cross-domain reference: always domain::types::Type
                    let parts: Vec<&str> = ref_.split('.').collect();
                    let domain_name = normalize_domain_name(&parts[0].to_snake_case());
                    format!("{}::types::{}", domain_name, parts[1])
                } else if from_types_submod {
                    // Same domain, inside types submodule: plain name
                    ref_.clone()
                } else {
                    // Same domain, inside methods/events submodule: types::Type
                    format!("types::{}", ref_)
                }
            }
        }
    }
}

pub fn normalize_domain_name(name: &str) -> String {
    let mut s = name.to_string();
    if s.contains("_d_o_m_") {
        s = s.replace("_d_o_m_", "_dom_");
    }
    if s == "service_worker" {
        s = "serviceworker".to_string();
    }
    s
}
