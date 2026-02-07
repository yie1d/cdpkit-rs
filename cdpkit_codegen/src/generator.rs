use crate::parser::{normalize_domain_name, Command, Domain, Event, Protocol, TypeDef};
use heck::{ToPascalCase, ToSnakeCase};
use std::collections::HashMap;

pub fn generate_code(protocols: &[Protocol]) -> String {
    let mut output = String::new();

    let now = {
        use std::time::SystemTime;
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let secs = d.as_secs();
        // Simple UTC timestamp
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;
        // Days since 1970-01-01
        let mut y = 1970i64;
        let mut remaining = days as i64;
        loop {
            let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
            if remaining < days_in_year { break; }
            remaining -= days_in_year;
            y += 1;
        }
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut m = 0usize;
        for &md in &month_days {
            if remaining < md { break; }
            remaining -= md;
            m += 1;
        }
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, m + 1, remaining + 1, hours, minutes, seconds)
    };
    output.push_str("// Auto-generated from Chrome DevTools Protocol\n");
    output.push_str(&format!("// Generated at: {}\n", now));
    output.push_str("// DO NOT EDIT MANUALLY  OvO\n\n");
    output.push_str("#![allow(dead_code, unused_imports, clippy::all)]\n\n");
    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push_str("use crate::{Method, CDP};\n\n");

    if let Some(protocol) = protocols.first() {
        output.push_str("/// CDP Protocol version (major.minor)\n");
        output.push_str(&format!(
            "pub const CDP_VERSION: &str = \"{}.{}\";\n\n",
            protocol.version.major, protocol.version.minor
        ));
    }

    let mut all_domains: Vec<Domain> = Vec::new();
    for protocol in protocols {
        all_domains.extend(protocol.domains.clone());
    }

    let type_map = build_type_map(&all_domains);

    for domain in &all_domains {
        output.push_str(&generate_domain(domain, &type_map));
        output.push('\n');
    }

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
    let module_name = normalize_domain_name(&domain.domain.to_snake_case());
    let has_types = !domain.types.is_empty();
    let has_responses = domain.commands.iter().any(|c| !c.returns.is_empty());

    output.push_str(&format!("pub mod {} {{\n", module_name));
    output.push_str("    use super::*;\n\n");

    // types submodule
    if has_types {
        output.push_str("    /// CDP type definitions for this domain.\n");
        output.push_str("    pub mod types {\n");
        output.push_str("        use super::super::*;\n\n");
        for type_def in &domain.types {
            output.push_str(&generate_type(type_def, &domain.domain, type_map));
            output.push('\n');
        }
        output.push_str("    }\n\n");
    }

    // responses submodule
    if has_responses {
        output.push_str("    /// Response types returned by methods in this domain.\n");
        output.push_str("    pub mod responses {\n");
        output.push_str("        use super::super::*;\n");
        if has_types {
            output.push_str("        use super::types;\n");
        }
        output.push('\n');
        for command in &domain.commands {
            if !command.returns.is_empty() {
                output.push_str(&generate_response(command, &domain.domain, type_map));
                output.push('\n');
            }
        }
        output.push_str("    }\n\n");
    }

    // methods submodule (commands)
    if !domain.commands.is_empty() {
        output.push_str("    /// CDP methods (commands) for this domain.\n");
        output.push_str("    pub mod methods {\n");
        output.push_str("        use super::super::*;\n");
        if has_types {
            output.push_str("        use super::types;\n");
        }
        if has_responses {
            output.push_str("        use super::responses;\n");
        }
        output.push('\n');
        for command in &domain.commands {
            output.push_str(&generate_command(command, &domain.domain, type_map));
            output.push('\n');
        }
        output.push_str("    }\n\n");
    }

    // events submodule
    if !domain.events.is_empty() {
        output.push_str("    /// CDP events for this domain.\n");
        output.push_str("    pub mod events {\n");
        output.push_str("        use super::super::*;\n");
        if has_types {
            output.push_str("        use super::types;\n");
        }
        output.push('\n');
        for event in &domain.events {
            output.push_str(&generate_event(event, &domain.domain, type_map));
            output.push('\n');
        }
        output.push_str("    }\n\n");
    }

    output.push_str("}\n");
    output
}

fn generate_type(type_def: &TypeDef, domain: &str, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let type_name = &type_def.id;

    if let Some(desc) = &type_def.description {
        output.push_str(&format!("        /// {}\n", escape_doc(desc)));
    }
    if type_def.experimental {
        output.push_str("        /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if type_def.deprecated {
        output.push_str("        #[deprecated]\n");
    }

    if !type_def.enum_.is_empty() {
        output.push_str("        #[derive(Debug, Clone, Serialize, Deserialize)]\n");
        output.push_str(&format!("        pub enum {} {{\n", type_name));
        for variant in &type_def.enum_ {
            output.push_str(&format!("            #[serde(rename = \"{}\")]\n", variant));
            let variant_name = sanitize_field_name(&variant.to_pascal_case());
            output.push_str(&format!("            {},\n", variant_name));
        }
        output.push_str("        }\n");

        output.push_str(&format!("        impl AsRef<str> for {} {{\n", type_name));
        output.push_str("            fn as_ref(&self) -> &str {\n");
        output.push_str("                match self {\n");
        for variant in &type_def.enum_ {
            let variant_name = sanitize_field_name(&variant.to_pascal_case());
            output.push_str(&format!(
                "                    Self::{} => \"{}\",\n",
                variant_name, variant
            ));
        }
        output.push_str("                }\n");
        output.push_str("            }\n");
        output.push_str("        }\n");

        output.push_str(&format!("        impl std::str::FromStr for {} {{\n", type_name));
        output.push_str("            type Err = String;\n");
        output.push_str("            fn from_str(s: &str) -> Result<Self, Self::Err> {\n");
        output.push_str("                match s {\n");
        for variant in &type_def.enum_ {
            let variant_name = sanitize_field_name(&variant.to_pascal_case());
            output.push_str(&format!(
                "                    \"{}\" => Ok(Self::{}),\n",
                variant, variant_name
            ));
        }
        output.push_str("                    _ => Err(s.to_string()),\n");
        output.push_str("                }\n");
        output.push_str("            }\n");
        output.push_str("        }\n");
    } else if !type_def.properties.is_empty() {
        output.push_str("        #[derive(Debug, Clone, Serialize, Deserialize)]\n");
        output.push_str("        #[serde(rename_all = \"camelCase\")]\n");
        output.push_str(&format!("        pub struct {} {{\n", type_name));
        for prop in &type_def.properties {
            if let Some(desc) = &prop.description {
                output.push_str(&format!("            /// {}\n", escape_doc(desc)));
            }
            if prop.experimental {
                output.push_str("            /// **EXPERIMENTAL**\n");
            }
            if prop.deprecated {
                output.push_str("            #[deprecated]\n");
            }
            if prop.optional {
                output.push_str(
                    "            #[serde(skip_serializing_if = \"Option::is_none\")]\n",
                );
            }
            let field_name = sanitize_field_name(&prop.name.to_snake_case());
            if needs_serde_rename(&prop.name, &field_name) {
                output.push_str(&format!("            #[serde(rename = \"{}\")]\n", prop.name));
            }
            let rust_type = prop.type_ref.to_rust_type(domain, type_map, true);
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
            output.push_str(&format!("            pub {}: {},\n", field_name, field_type));
        }
        output.push_str("        }\n");
    } else {
        let rust_type = if let Some(base_type) = &type_def.type_ {
            match base_type.as_str() {
                "string" => "String".to_string(),
                "integer" => "i64".to_string(),
                "number" => "f64".to_string(),
                "boolean" => "bool".to_string(),
                "object" => "serde_json::Value".to_string(),
                "any" => "serde_json::Value".to_string(),
                "array" => {
                    if let Some(items) = &type_def.items {
                        let item_type = items.to_rust_type(domain, type_map, true);
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
        output.push_str(&format!("        pub type {} = {};\n", type_name, rust_type));
    }

    output
}

fn generate_response(
    command: &Command,
    domain: &str,
    type_map: &HashMap<String, String>,
) -> String {
    let mut output = String::new();
    let struct_name = format!("{}Response", command.name.to_pascal_case());

    output.push_str("        #[derive(Debug, Clone, Deserialize)]\n");
    output.push_str("        #[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("        pub struct {} {{\n", struct_name));
    for ret in &command.returns {
        if let Some(desc) = &ret.description {
            output.push_str(&format!("            /// {}\n", escape_doc(desc)));
        }
        if ret.experimental {
            output.push_str("            /// **EXPERIMENTAL**\n");
        }
        if ret.deprecated {
            output.push_str("            #[deprecated]\n");
        }
        let field_name = sanitize_field_name(&ret.name.to_snake_case());
        if needs_serde_rename(&ret.name, &field_name) {
            output.push_str(&format!("            #[serde(rename = \"{}\")]\n", ret.name));
        }
        let rust_type = ret.type_ref.to_rust_type(domain, type_map, false);
        let field_type = if ret.optional || ret.experimental {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };
        output.push_str(&format!("            pub {}: {},\n", field_name, field_type));
    }
    output.push_str("        }\n");

    output
}

fn generate_command(
    command: &Command,
    domain: &str,
    type_map: &HashMap<String, String>,
) -> String {
    let mut output = String::new();
    let struct_name = command.name.to_pascal_case();
    let method_name = format!("{}.{}", domain, command.name);

    if let Some(desc) = &command.description {
        output.push_str(&format!("        /// {}\n", escape_doc(desc)));
    }
    if command.experimental {
        output.push_str("        /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if command.deprecated {
        output.push_str("        #[deprecated]\n");
    }
    output.push_str("        #[derive(Debug, Clone, Serialize)]\n");
    output.push_str("        #[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("        pub struct {} {{\n", struct_name));

    for param in &command.parameters {
        if let Some(desc) = &param.description {
            output.push_str(&format!("            /// {}\n", escape_doc(desc)));
        }
        if param.experimental {
            output.push_str("            /// **EXPERIMENTAL**\n");
        }
        if param.deprecated {
            output.push_str("            #[deprecated]\n");
        }
        if param.optional {
            output.push_str(
                "            #[serde(skip_serializing_if = \"Option::is_none\")]\n",
            );
        }
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        if needs_serde_rename(&param.name, &field_name) {
            output.push_str(&format!("            #[serde(rename = \"{}\")]\n", param.name));
        }
        let rust_type = param.type_ref.to_rust_type(domain, type_map, false);
        let field_type = if param.optional {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };
        output.push_str(&format!("            {}: {},\n", field_name, field_type));
    }

    output.push_str("        }\n\n");

    // Constructor and builder methods
    output.push_str(&format!("        impl {} {{\n", struct_name));

    let required_params: Vec<_> = command.parameters.iter().filter(|p| !p.optional).collect();
    if required_params.is_empty() {
        output.push_str("            pub fn new() -> Self {\n");
        output.push_str("                Self {\n");
        for param in &command.parameters {
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            output.push_str(&format!("                    {}: None,\n", field_name));
        }
        output.push_str("                }\n");
        output.push_str("            }\n");
    } else {
        output.push_str("            pub fn new(");
        for (i, param) in required_params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            let rust_type = param.type_ref.to_rust_type(domain, type_map, false);
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            output.push_str(&format!("{}: impl Into<{}>", field_name, rust_type));
        }
        output.push_str(") -> Self {\n");
        output.push_str("                Self {\n");
        for param in &command.parameters {
            let field_name = sanitize_field_name(&param.name.to_snake_case());
            if param.optional {
                output.push_str(&format!("                    {}: None,\n", field_name));
            } else {
                output.push_str(&format!(
                    "                    {}: {}.into(),\n",
                    field_name, field_name
                ));
            }
        }
        output.push_str("                }\n");
        output.push_str("            }\n");
    }

    for param in command.parameters.iter().filter(|p| p.optional) {
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        let with_name = format!("with_{}", field_name);
        let rust_type = param.type_ref.to_rust_type(domain, type_map, false);
        output.push_str(&format!(
            "\n            pub fn {}(mut self, {}: impl Into<{}>) -> Self {{\n",
            with_name, field_name, rust_type
        ));
        output.push_str(&format!(
            "                self.{} = Some({}.into());\n",
            field_name, field_name
        ));
        output.push_str("                self\n");
        output.push_str("            }\n");
    }

    output.push_str("        }\n\n");

    // Method trait implementation
    let result_type = if command.returns.is_empty() {
        "()".to_string()
    } else {
        format!("responses::{}Response", struct_name)
    };

    output.push_str(&format!("        impl Method for {} {{\n", struct_name));
    output.push_str(&format!("            type Response = {};\n", result_type));
    output.push_str(&format!(
        "            const METHOD: &'static str = \"{}\";\n",
        method_name
    ));
    output.push_str("        }\n");

    output
}

fn generate_event(event: &Event, domain: &str, type_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let struct_name = event.name.to_pascal_case();
    let method_name = format!("{}.{}", domain, event.name);

    if let Some(desc) = &event.description {
        output.push_str(&format!("        /// {}\n", escape_doc(desc)));
    }
    if event.experimental {
        output.push_str("        /// **EXPERIMENTAL**: This feature is experimental and may change or be removed.\n");
    }
    if event.deprecated {
        output.push_str("        #[deprecated]\n");
    }
    output.push_str("        #[derive(Debug, Clone, Deserialize)]\n");
    output.push_str("        #[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("        pub struct {} {{\n", struct_name));

    for param in &event.parameters {
        if let Some(desc) = &param.description {
            output.push_str(&format!("            /// {}\n", escape_doc(desc)));
        }
        if param.experimental {
            output.push_str("            /// **EXPERIMENTAL**\n");
        }
        if param.deprecated {
            output.push_str("            #[deprecated]\n");
        }
        let field_name = sanitize_field_name(&param.name.to_snake_case());
        if needs_serde_rename(&param.name, &field_name) {
            output.push_str(&format!("            #[serde(rename = \"{}\")]\n", param.name));
        }
        let rust_type = param.type_ref.to_rust_type(domain, type_map, false);
        let field_type = if param.optional {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };
        output.push_str(&format!("            pub {}: {},\n", field_name, field_type));
    }

    output.push_str("        }\n\n");

    output.push_str(&format!("        impl {} {{\n", struct_name));
    output.push_str("            pub fn subscribe(cdp: &CDP) -> std::pin::Pin<Box<dyn futures::stream::Stream<Item = Self> + Send>> {\n");
    output.push_str(&format!(
        "                cdp.inner.event_stream(\"{}\")\n",
        method_name
    ));
    output.push_str("            }\n");
    output.push_str("        }\n");

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

/// Check if a field needs an explicit `#[serde(rename)]` attribute.
/// With `rename_all = "camelCase"` on the struct, serde converts snake_case field names
/// to camelCase. We only need explicit rename when that conversion doesn't match the
/// original JSON property name.
fn needs_serde_rename(original_name: &str, field_name: &str) -> bool {
    if field_name == original_name {
        return false;
    }
    // Simulate serde's snake_case -> camelCase conversion
    let mut camel = String::new();
    let mut capitalize_next = false;
    for ch in field_name.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            camel.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            camel.push(ch);
        }
    }
    camel != original_name
}
