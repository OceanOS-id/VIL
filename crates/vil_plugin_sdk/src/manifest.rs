// =============================================================================
// Plugin Manifest — Declarative plugin metadata for discovery and validation
// =============================================================================
//
// PluginManifest is a serializable description of a plugin's identity,
// capabilities, config schema, and requirements. Used for:
//   - Plugin discovery (scanning plugin directory)
//   - Compatibility checking before loading
//   - Config validation (JSON Schema)
//   - Admin UI generation
//
// Example manifest.json:
//   {
//     "name": "my-plugin",
//     "version": "1.0.0",
//     "description": "My awesome plugin",
//     "author": "Community",
//     "license": "MIT",
//     "provides": ["service:my-svc"],
//     "requires": ["vil-llm >=0.1"],
//     "config_schema": {
//       "api_key": { "type": "string", "required": true, "secret": true },
//       "timeout_ms": { "type": "integer", "default": 5000 }
//     }
//   }

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Declarative plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (must match VilPlugin::id())
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Plugin author
    #[serde(default)]
    pub author: String,
    /// License identifier (SPDX)
    #[serde(default)]
    pub license: String,
    /// Homepage / repository URL
    #[serde(default)]
    pub homepage: String,
    /// What this plugin provides (e.g., "service:my-svc", "resource:LlmProvider")
    #[serde(default)]
    pub provides: Vec<String>,
    /// What this plugin requires (e.g., "vil-llm >=0.1")
    #[serde(default)]
    pub requires: Vec<String>,
    /// Configuration schema (field name → schema)
    #[serde(default)]
    pub config_schema: HashMap<String, ConfigFieldSchema>,
    /// Minimum VIL version required
    #[serde(default)]
    pub min_vil_version: Option<String>,
}

/// Schema for a single configuration field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFieldSchema {
    /// Field type: string, integer, boolean, array, object
    #[serde(rename = "type")]
    pub field_type: String,
    /// Whether this field is required
    #[serde(default)]
    pub required: bool,
    /// Default value (JSON)
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Description for documentation / admin UI
    #[serde(default)]
    pub description: String,
    /// Whether this field contains a secret (masked in admin UI)
    #[serde(default)]
    pub secret: bool,
}

impl PluginManifest {
    /// Create a new manifest with required fields.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: String::new(),
            license: String::new(),
            homepage: String::new(),
            provides: Vec::new(),
            requires: Vec::new(),
            config_schema: HashMap::new(),
            min_vil_version: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.license = license.into();
        self
    }

    pub fn provides(mut self, cap: impl Into<String>) -> Self {
        self.provides.push(cap.into());
        self
    }

    pub fn requires(mut self, dep: impl Into<String>) -> Self {
        self.requires.push(dep.into());
        self
    }

    pub fn config_field(mut self, name: impl Into<String>, schema: ConfigFieldSchema) -> Self {
        self.config_schema.insert(name.into(), schema);
        self
    }

    pub fn min_vil(mut self, version: impl Into<String>) -> Self {
        self.min_vil_version = Some(version.into());
        self
    }

    /// Load manifest from a JSON file.
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Validate the manifest (basic checks).
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("name is required".into());
        }
        if self.version.is_empty() {
            errors.push("version is required".into());
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push("name must contain only alphanumeric, '-', or '_' characters".into());
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

impl ConfigFieldSchema {
    pub fn string() -> Self {
        Self { field_type: "string".into(), required: false, default: None, description: String::new(), secret: false }
    }
    pub fn integer() -> Self {
        Self { field_type: "integer".into(), required: false, default: None, description: String::new(), secret: false }
    }
    pub fn boolean() -> Self {
        Self { field_type: "boolean".into(), required: false, default: None, description: String::new(), secret: false }
    }

    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn secret(mut self) -> Self { self.secret = true; self }
    pub fn description(mut self, desc: impl Into<String>) -> Self { self.description = desc.into(); self }
    pub fn default_value(mut self, val: serde_json::Value) -> Self { self.default = Some(val); self }
}
