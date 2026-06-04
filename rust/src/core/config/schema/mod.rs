//! Auto-generated config schema from `Config` struct metadata.
//!
//! Used by `lean-ctx config schema` to emit JSON and by
//! `lean-ctx config validate` to check user config.toml files.

use serde::Serialize;
use std::collections::BTreeMap;
mod sections_advanced;
mod sections_core;
mod sections_features;

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSchema {
    pub version: u32,
    pub sections: BTreeMap<String, SectionSchema>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionSchema {
    pub description: String,
    pub keys: BTreeMap<String, KeySchema>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeySchema {
    #[serde(rename = "type")]
    pub ty: String,
    pub default: serde_json::Value,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_override: Option<String>,
}

fn clean_f32(v: f32) -> serde_json::Value {
    let clean: f64 = format!("{v}").parse().unwrap_or(v as f64);
    serde_json::json!(clean)
}

fn key(ty: &str, default: serde_json::Value, desc: &str) -> KeySchema {
    KeySchema {
        ty: ty.to_string(),
        default,
        description: desc.to_string(),
        values: None,
        env_override: None,
    }
}

fn key_enum(values: &[&str], default: &str, desc: &str) -> KeySchema {
    KeySchema {
        ty: "enum".to_string(),
        default: serde_json::Value::String(default.to_string()),
        description: desc.to_string(),
        values: Some(values.iter().map(ToString::to_string).collect()),
        env_override: None,
    }
}

fn key_with_env(ty: &str, default: serde_json::Value, desc: &str, env: &str) -> KeySchema {
    KeySchema {
        ty: ty.to_string(),
        default,
        description: desc.to_string(),
        values: None,
        env_override: Some(env.to_string()),
    }
}

fn key_enum_with_env(values: &[&str], default: &str, desc: &str, env: &str) -> KeySchema {
    KeySchema {
        ty: "enum".to_string(),
        default: serde_json::Value::String(default.to_string()),
        description: desc.to_string(),
        values: Some(values.iter().map(ToString::to_string).collect()),
        env_override: Some(env.to_string()),
    }
}

impl ConfigSchema {
    pub fn generate() -> Self {
        let mut sections = BTreeMap::new();
        sections_core::build(&mut sections);
        sections_features::build(&mut sections);
        sections_advanced::build(&mut sections);

        ConfigSchema {
            version: 1,
            sections,
        }
    }

    /// Looks up a key schema by its dot-separated TOML path.
    /// Returns `None` if the key is not part of the schema.
    pub fn lookup(&self, key: &str) -> Option<&KeySchema> {
        if let Some(dot_pos) = key.find('.') {
            let section = &key[..dot_pos];
            let field = &key[dot_pos + 1..];
            self.sections.get(section)?.keys.get(field)
        } else {
            self.sections.get("root")?.keys.get(key)
        }
    }

    /// All known TOML keys (dot-separated) for validation.
    pub fn known_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for (section, schema) in &self.sections {
            if section == "root" {
                for key_name in schema.keys.keys() {
                    keys.push(key_name.clone());
                }
            } else {
                if schema.keys.is_empty() {
                    keys.push(section.clone());
                }
                for key_name in schema.keys.keys() {
                    keys.push(format!("{section}.{key_name}"));
                }
            }
        }
        keys
    }
}
