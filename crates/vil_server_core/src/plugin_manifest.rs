// =============================================================================
// VIL Server — Plugin Manifest & Registry
// =============================================================================
//
// Defines the plugin manifest format, plugin state machine, and the
// persistent plugin registry that tracks installed plugins on disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Plugin tier — determines trust level and UI presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginTier {
    /// Signed by VIL Core Team
    Official,
    /// Community-contributed, user trust model
    Community,
}

/// Plugin type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Database,
    Cache,
    MessageQueue,
    Auth,
    Observability,
    Custom,
}

/// Plugin state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginState {
    /// Installed on disk but not activated
    Installed,
    /// Active and serving requests
    Enabled,
    /// Explicitly disabled by user
    Disabled,
    /// Error during enable/health check
    Error,
}

/// Config field schema — describes one configuration parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    /// Field type: string, integer, enum, boolean
    #[serde(rename = "type")]
    pub field_type: String,
    /// Human-readable label
    #[serde(default)]
    pub label: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Whether this field is required
    #[serde(default)]
    pub required: bool,
    /// Whether this field contains a secret (password, token)
    #[serde(default)]
    pub secret: bool,
    /// Default value (JSON)
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Enum values (for type=enum)
    #[serde(default)]
    pub values: Vec<String>,
    /// Min value (for type=integer)
    #[serde(default)]
    pub min: Option<i64>,
    /// Max value (for type=integer)
    #[serde(default)]
    pub max: Option<i64>,
    /// Placeholder text
    #[serde(default)]
    pub placeholder: String,
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_30")]
    pub interval_secs: u64,
    #[serde(default = "default_5")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub query: String,
}

fn default_true() -> bool {
    true
}
fn default_30() -> u64 {
    30
}
fn default_5() -> u64 {
    5
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 30,
            timeout_secs: 5,
            query: "SELECT 1".into(),
        }
    }
}

/// Metric definition in manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDef {
    pub name: String,
    #[serde(rename = "type")]
    pub metric_type: String,
    #[serde(default)]
    pub label: String,
}

/// Admin UI hints.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminUiHints {
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub color: String,
}

/// Signature for official plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    pub algorithm: String,
    pub public_key: String,
    pub signature: String,
}

/// Plugin manifest — the complete description of a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    #[serde(default = "default_community")]
    pub tier: PluginTier,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub homepage: String,

    #[serde(default)]
    pub signature: Option<PluginSignature>,

    #[serde(default)]
    pub config_schema: HashMap<String, ConfigField>,

    #[serde(default)]
    pub health_check: HealthCheckConfig,

    #[serde(default)]
    pub metrics: Vec<MetricDef>,

    #[serde(default)]
    pub admin_ui: AdminUiHints,

    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

fn default_community() -> PluginTier {
    PluginTier::Community
}

/// Plugin runtime state — persisted to state.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRuntimeState {
    pub name: String,
    pub state: PluginState,
    pub enabled_at: Option<u64>,
    pub disabled_at: Option<u64>,
    pub last_health_check: Option<u64>,
    pub last_health_status: Option<String>,
    pub error_message: Option<String>,
    pub config_version: u64,
}

impl PluginRuntimeState {
    pub fn new_installed(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: PluginState::Installed,
            enabled_at: None,
            disabled_at: None,
            last_health_check: None,
            last_health_status: None,
            error_message: None,
            config_version: 0,
        }
    }
}

/// Installed plugins registry — persisted to registry.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsRegistry {
    pub plugins: HashMap<String, PluginRegistryEntry>,
}

/// Entry in the plugins registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistryEntry {
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub tier: PluginTier,
    pub install_path: String,
    pub installed_at: u64,
}

impl PluginsRegistry {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize registry: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("Failed to write registry: {}", e))
    }

    pub fn register(&mut self, entry: PluginRegistryEntry) {
        self.plugins.insert(entry.name.clone(), entry);
    }

    pub fn unregister(&mut self, name: &str) {
        self.plugins.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<&PluginRegistryEntry> {
        self.plugins.get(name)
    }

    pub fn list(&self) -> Vec<&PluginRegistryEntry> {
        self.plugins.values().collect()
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}
