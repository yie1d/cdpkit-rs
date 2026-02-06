use crate::parser::{Command, Domain, Event, Protocol, TypeDef};
use heck::{ToPascalCase, ToSnakeCase};
use std::collections::HashMap;

pub fn generate_code(protocols: &[Protocol]) -> String {
    let mut output = String::new();

    // Header with generation date
    let now = chrono::Local::now();
    output.push_str("// Auto-generated from Chrome DevTools Protocol\n");
    output.push_str(&format!(
        "// Generated at: {}\n",
        now.format("%Y-%m-%d %H:%M:%S %Z")
    ));
    output.push_str("// DO NOT EDIT MANUALLY  OvO\n\n");
    output.push_str("#![allow(dead_code, unused_imports, clippy::all)]\n\n");
    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push_str("use crate::{Command, CDP};\n");
    output.push_str("use futures::stream::Stream;\n");
    output.push_str("use std::sync::Arc;\n\n");

    // Generate protocol version constants from the first protocol
    if let Some(protocol) = protocols.first() {
        output.push_str("/// CDP Protocol version (major.minor)\n");
        output.push_str(&format!(
            "pub const CDP_VERSION: &str = \"{}.{}\";\n\n",
            protocol.version.major, protocol.version.minor
        ));
    }

    // Collect all domains
    let mut all_domains: Vec<Domain> = Vec::new();
    for protocol in protocols {
        all_domains.extend(protocol.domains.clone());
    }

    // Build type map
    let type_map = build_type_map(&all_domains);

    // Generate each domain
    for domain in &all_domains {
        output.push_str(&generate_domain(domain, &type_map));
        output.push('\n');
    }

    // Generate common type conversions
    output.push_str(&generate_type_conversions(&all_domains));

    output
}

fn build_type_map(domains: &[Domain]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for domain in domains {
        for type_def in &domain.types {
            let key = format!("{}.{}", domain.domain, type_def.id);
            map.insert(key, type_def.id.clone());
        }
    }
    map
}

fn generate_domain(domain: &Domain, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let mut module_name = domain.domain.to_snake_case();
    // Fix special cases
    if module_name.contains("_d_o_m_") {
        module_name = module_name.replace("_d_o_m_", "_dom_");
    }
    if module_name == "service_worker" {
        module_name = "serviceworker".to_string();
    }

    output.push_str(&format!("pub mod {} {{\n", module_name));
    output.push_str("    use super::*;\n\n");

    // Generate types
    for type_def in &domain.types {
        output.push_str(&generate_type(type_def, &domain.domain, type_map));
        output.push('\n');
    }

    // Generate commands
    for command in &domain.commands {
        output.push_str(&generate_command(command, &domain.domain, type_map));
        output.push('\n');
    }

    // Generate events
    for event in &domain.events {
        output.push_str(&generate_event(event, &domain.domain, type_map));
        output.push('\n');
    }

    output.push_str("}\n");
    output
}

fn generate_type(type_def: &TypeDef, domain: &str, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let type_name = &type_def.id;

    if let Some(desc) = &type_def.description {
        output.push_str(&format!("    /// {}\n", escape_doc(desc)));
    }

    if type_def.experimental {
        output.push_str("    /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if type_def.deprecated {
        output.push_str("    #[deprecated]\n");
    }

    if !type_def.enum_.is_empty() {
        // Enum type
        output.push_str(&format!(
            "    #[derive(Debug, Clone, Serialize, Deserialize)]\n"
        ));
        output.push_str(&format!("    pub enum {} {{\n", type_name));
        for variant in &type_def.enum_ {
            output.push_str(&format!("        #[serde(rename = \"{}\")]\n", variant));
            let variant_name = sanitize_field_name(&variant.to_pascal_case());
            output.push_str(&format!("        {},\n", variant_name));
        }
        output.push_str("    }\n");
        
        // Add From<&str> and From<String> implementations for better UX
        output.push_str(&format!("    impl From<&str> for {} {{\n", type_name));
        output.push_str("        fn from(s: &str) -> Self {\n");
        output.push_str("            match s {\n");
        for variant in &type_def.enum_ {
            let variant_name = sanitize_field_name(&variant.to_pascal_case());
            output.push_str(&format!("                \"{}\" => Self::{},\n", variant, variant_name));
        }
        output.push_str(&format!("                _ => Self::{},\n", sanitize_field_name(&type_def.enum_[0].to_pascal_case())));
        output.push_str("            }\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        
        output.push_str(&format!("    impl From<String> for {} {{\n", type_name));
        output.push_str("        fn from(s: String) -> Self {\n");
        output.push_str("            Self::from(s.as_str())\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
    } else if !type_def.properties.is_empty() {
        // Struct type
        output.push_str(&format!(
            "    #[derive(Debug, Clone, Serialize, Deserialize)]\n"
        ));
        output.push_str("    #[serde(rename_all = \"camelCase\")]\n");
        output.push_str(&format!("    pub struct {} {{\n", type_name));
        for prop in &type_def.properties {
            if let Some(desc) = &prop.description {
                output.push_str(&format!("        /// {}\n", escape_doc(desc)));
            }
            if prop.experimental {
                output.push_str("        /// **EXPERIMENTAL**\n");
            }
            if prop.deprecated {
                output.push_str("        #[deprecated]\n");
            }
            if prop.optional {
                output.push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
            }
            let field_name = sanitize_field_name(&prop.name.to_snake_case());
            if field_name != prop.name {
                output.push_str(&format!("        #[serde(rename = \"{}\")]\n", prop.name));
            }
            let rust_type = prop.type_ref.to_rust_type(domain, type_map);
            // Box recursive types
            let rust_type = if rust_type == *type_name {
                format!("Box<{}>", rust_type)
            } else {
                rust_type
            };
            let field_type = if prop.optional {
                format!("Option<{}>", rust_type)
            } else {
                rust_type
            };
            output.push_str(&format!("        pub {}: {},\n", field_name, field_type));
        }
        output.push_str("    }\n");
    } else {
        // Type alias
        let rust_type = if let Some(base_type) = &type_def.type_ {
            match base_type.as_str() {
                "string" => "String".to_string(),
                "integer" => "i64".to_string(),
                "number" => "f64".to_string(),
                "boolean" => "bool".to_string(),
                "object" => "serde_json::Value".to_string(),
                "any" => "serde_json::Value".to_string(),
                "array" => {
                    // Handle array types
                    if let Some(items) = &type_def.items {
                        let item_type = items.to_rust_type(domain, type_map);
                        format!("Vec<{}>", item_type)
                    } else {
                        "Vec<serde_json::Value>".to_string()
                    }
                }
                _ => "String".to_string(),
            }
        } else {
            "String".to_string()
        };
        output.push_str(&format!("    pub type {} = {};\n", type_name, rust_type));
    }

    output
}

fn generate_command(command: &Command, domain: &str, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let struct_name = command.name.to_pascal_case();
    let method_name = format!("{}.{}", domain, command.name);

    if let Some(desc) = &command.description {
        output.push_str(&format!("    /// {}\n", escape_doc(desc)));
    }
    if command.experimental {
        output.push_str("    /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if command.deprecated {
        output.push_str("    #[deprecated]\n");
    }
    output.push_str(&format!("    #[derive(Debug, Clone, Serialize)]\n"));
    output.push_str("    #[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("    pub struct {} {{\n", struct_name));

    for param in &command.parameters {
        if let Some(desc) = &param.description {
            output.push_str(&format!("        /// {}\n", escape_doc(desc)));
        }
        if param.experimental {
            output.push_str("        /// **EXPERIMENTAL**\n");
        }
        if param.deprecated {
            output.push_str("        #[deprecated]\n");
        }
        if param.optional {
            output.push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
        }
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        if field_name != param.name {
            output.push_str(&format!("        #[serde(rename = \"{}\")]\n", param.name));
        }
        let rust_type = param.type_ref.to_rust_type(domain, type_map);
        let field_type = if param.optional {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };
        output.push_str(&format!("        {}: {},\n", field_name, field_type));
    }

    output.push_str("    }\n\n");

    // Constructor and builder methods
    output.push_str(&format!("    impl {} {{\n", struct_name));

    // new() method
    let required_params: Vec<_> = command.parameters.iter().filter(|p| !p.optional).collect();
    if required_params.is_empty() {
        output.push_str("        pub fn new() -> Self {\n");
        output.push_str("            Self {\n");
        for param in &command.parameters {
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            output.push_str(&format!("                {}: None,\n", field_name));
        }
        output.push_str("            }\n");
        output.push_str("        }\n");
    } else {
        output.push_str("        pub fn new(");
        for (i, param) in required_params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            let rust_type = param.type_ref.to_rust_type(domain, type_map);
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            output.push_str(&format!("{}: impl Into<{}>", field_name, rust_type));
        }
        output.push_str(") -> Self {\n");
        output.push_str("            Self {\n");
        for param in &command.parameters {
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            if param.optional {
                output.push_str(&format!("                {}: None,\n", field_name));
            } else {
                output.push_str(&format!(
                    "                {}: {}.into(),\n",
                    field_name, field_name
                ));
            }
        }
        output.push_str("            }\n");
        output.push_str("        }\n");
    }

    // with_* methods for optional parameters
    for param in command.parameters.iter().filter(|p| p.optional) {
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        let method_name = format!("with_{}", field_name);
        let rust_type = param.type_ref.to_rust_type(domain, type_map);
        output.push_str(&format!(
            "\n        pub fn {}(mut self, {}: impl Into<{}>) -> Self {{\n",
            method_name, field_name, rust_type
        ));
        output.push_str(&format!(
            "            self.{} = Some({}.into());\n",
            field_name, field_name
        ));
        output.push_str("            self\n");
        output.push_str("        }\n");
    }

    output.push_str("    }\n\n");

    // Command trait implementation
    let result_type = if command.returns.is_empty() {
        "()".to_string()
    } else {
        format!("{}Result", struct_name)
    };

    output.push_str(&format!("    impl Command for {} {{\n", struct_name));
    output.push_str(&format!("        type Response = {};\n", result_type));
    output.push_str(&format!(
        "        const METHOD: &'static str = \"{}\";\n",
        method_name
    ));
    output.push_str("    }\n");

    // Result type
    if !command.returns.is_empty() {
        output.push_str(&format!("\n    #[derive(Debug, Clone, Deserialize)]\n"));
        output.push_str("    #[serde(rename_all = \"camelCase\")]\n");
        output.push_str(&format!("    pub struct {}Result {{\n", struct_name));
        for ret in &command.returns {
            if let Some(desc) = &ret.description {
                output.push_str(&format!("        /// {}\n", escape_doc(desc)));
            }
            if ret.experimental {
                output.push_str("        /// **EXPERIMENTAL**\n");
            }
            if ret.deprecated {
                output.push_str("        #[deprecated]\n");
            }
            let field_name = sanitize_field_name(&ret.name.to_snake_case());
            if field_name != ret.name {
                output.push_str(&format!("        #[serde(rename = \"{}\")]\n", ret.name));
            }
            let rust_type = ret.type_ref.to_rust_type(domain, type_map);
            // Experimental fields should be optional as they may not always be present
            let field_type = if ret.optional || ret.experimental {
                format!("Option<{}>", rust_type)
            } else {
                rust_type
            };
            output.push_str(&format!("        pub {}: {},\n", field_name, field_type));
        }
        output.push_str("    }\n");
    }

    output
}

fn generate_event(event: &Event, domain: &str, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let struct_name = event.name.to_pascal_case();
    let method_name = format!("{}.{}", domain, event.name);

    if let Some(desc) = &event.description {
        output.push_str(&format!("    /// {}\n", escape_doc(desc)));
    }
    if event.experimental {
        output.push_str("    /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if event.deprecated {
        output.push_str("    #[deprecated]\n");
    }
    output.push_str(&format!("    #[derive(Debug, Clone, Deserialize)]\n"));
    output.push_str("    #[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("    pub struct {} {{\n", struct_name));

    for param in &event.parameters {
        if let Some(desc) = &param.description {
            output.push_str(&format!("        /// {}\n", escape_doc(desc)));
        }
        if param.experimental {
            output.push_str("        /// **EXPERIMENTAL**\n");
        }
        if param.deprecated {
            output.push_str("        #[deprecated]\n");
        }
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        if field_name != param.name {
            output.push_str(&format!("        #[serde(rename = \"{}\")]\n", param.name));
        }
        let rust_type = param.type_ref.to_rust_type(domain, type_map);
        let field_type = if param.optional {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };
        output.push_str(&format!("        pub {}: {},\n", field_name, field_type));
    }

    output.push_str("    }\n\n");

    // subscribe() method
    output.push_str(&format!("    impl {} {{\n", struct_name));
    output.push_str("        pub fn subscribe(cdp: &CDP) -> std::pin::Pin<Box<dyn futures::stream::Stream<Item = Self> + Send>> {\n");
    output.push_str(&format!(
        "            cdp.inner.event_stream(\"{}\")\n",
        method_name
    ));
    output.push_str("        }\n");
    output.push_str("    }\n");

    output
}

fn sanitize_field_name(name: &str) -> String {
    match name {
        "type" => "type_".to_string(),
        "ref" => "ref_".to_string(),
        "mod" => "mod_".to_string(),
        "use" => "use_".to_string(),
        "loop" => "loop_".to_string(),
        "move" => "move_".to_string(),
        "match" => "match_".to_string(),
        "self" => "self_".to_string(),
        "Self" => "Self_".to_string(),
        "override" => "override_".to_string(),
        _ => name.to_string(),
    }
}

fn escape_doc(s: &str) -> String {
    s.replace('\n', " ").replace("  ", " ")
}

fn generate_type_conversions(domains: &[Domain]) -> String {
    let mut output = String::new();

    output.push_str("/// Common type conversions\n");
    output.push_str("///\n");
    output.push_str("/// Note: Since RequestId types are type aliases to String,\n");
    output.push_str("/// they can be used interchangeably without explicit conversion.\n");
    output.push_str("pub mod conversions {\n");
    output.push_str("    use super::*;\n\n");

    // Find RequestId types across domains
    let mut request_id_domains = Vec::new();
    for domain in domains {
        for type_def in &domain.types {
            if type_def.id == "RequestId" {
                let mut domain_name = domain.domain.to_snake_case();
                // Fix special cases
                if domain_name.contains("_d_o_m_") {
                    domain_name = domain_name.replace("_d_o_m_", "_dom_");
                }
                if domain_name == "service_worker" {
                    domain_name = "serviceworker".to_string();
                }
                request_id_domains.push(domain_name);
            }
        }
    }

    // Generate conversion function
    if !request_id_domains.is_empty() {
        output.push_str("    /// Convert between RequestId types from different domains.\n");
        output.push_str("    ///\n");
        output.push_str("    /// Since they are all String aliases, this is a no-op but provides type clarity.\n");
        output.push_str("    ///\n");
        output.push_str("    /// Available RequestId types:\n");
        for domain in &request_id_domains {
            output.push_str(&format!("    /// - `{}::RequestId`\n", domain));
        }
        output.push_str("    #[inline]\n");
        output.push_str("    pub fn convert_request_id<T: Into<String>>(id: T) -> String {\n");
        output.push_str("        id.into()\n");
        output.push_str("    }\n");
    }

    output.push_str("}\n");
    output
}
