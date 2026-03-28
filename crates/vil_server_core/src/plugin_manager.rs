// =============================================================================
// VIL Server — Plugin Manager
// =============================================================================
//
// Central orchestrator for plugin lifecycle:
//   install → enable → configure → health check → disable → remove
//
// Manages the plugin registry, config persistence, and state transitions.

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::plugin_manifest::*;
use crate::secrets::SecretResolver;

/// Plugin Manager — manages all installed plugins.
pub struct PluginManager {
    /// Base directory for plugin storage (~/.vil/plugins/)
    plugins_dir: PathBuf,
    /// Plugin registry (persisted to registry.json)
    registry: std::sync::RwLock<PluginsRegistry>,
    /// Runtime state per plugin
    states: DashMap<String, PluginRuntimeState>,
    /// Loaded manifests
    manifests: DashMap<String, PluginManifest>,
    /// Plugin configurations (resolved secrets)
    configs: DashMap<String, serde_json::Value>,
    /// Secret resolver
    secrets: Arc<SecretResolver>,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new(plugins_dir: &Path, secrets: Arc<SecretResolver>) -> Self {
        let registry_path = plugins_dir.join("registry.json");
        let registry = PluginsRegistry::load(&registry_path);

        let mgr = Self {
            plugins_dir: plugins_dir.to_path_buf(),
            registry: std::sync::RwLock::new(registry),
            states: DashMap::new(),
            manifests: DashMap::new(),
            configs: DashMap::new(),
            secrets,
        };

        // Load existing plugins from disk
        mgr.load_installed_plugins();
        mgr
    }

    /// Load all installed plugins from disk.
    fn load_installed_plugins(&self) {
        let registry = self.registry.read().unwrap();
        for (name, _entry) in &registry.plugins {
            let plugin_dir = self.plugins_dir.join(name);

            // Load manifest
            let manifest_path = plugin_dir.join("manifest.json");
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                    self.manifests.insert(name.clone(), manifest);
                }
            }

            // Load state
            let state_path = plugin_dir.join("state.json");
            let state = std::fs::read_to_string(&state_path)
                .ok()
                .and_then(|s| serde_json::from_str::<PluginRuntimeState>(&s).ok())
                .unwrap_or_else(|| PluginRuntimeState::new_installed(name));
            self.states.insert(name.clone(), state);

            // Load config
            let config_path = plugin_dir.join("config.yaml");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_yaml::from_str::<serde_json::Value>(&content) {
                    self.configs.insert(name.clone(), config);
                }
            }

            // debug-level: skip vil_log
        }
    }

    /// Install a plugin from manifest.
    pub fn install(&self, manifest: PluginManifest) -> Result<(), String> {
        let name = manifest.name.clone();
        let plugin_dir = self.plugins_dir.join(&name);

        // Create plugin directory
        std::fs::create_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;

        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        std::fs::write(plugin_dir.join("manifest.json"), &manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        // Write initial state
        let state = PluginRuntimeState::new_installed(&name);
        let state_json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        std::fs::write(plugin_dir.join("state.json"), &state_json)
            .map_err(|e| format!("Failed to write state: {}", e))?;

        // Write default config
        let default_config = self.build_default_config(&manifest);
        let config_yaml = serde_yaml::to_string(&default_config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(plugin_dir.join("config.yaml"), &config_yaml)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Update registry
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = PluginRegistryEntry {
            name: name.clone(),
            version: manifest.version.clone(),
            plugin_type: manifest.plugin_type.clone(),
            tier: manifest.tier.clone(),
            install_path: plugin_dir.to_string_lossy().to_string(),
            installed_at: timestamp,
        };

        {
            let mut registry = self.registry.write().unwrap();
            registry.register(entry);
            let _ = registry.save(&self.plugins_dir.join("registry.json"));
        }

        self.manifests.insert(name.clone(), manifest);
        self.states.insert(name.clone(), state);
        self.configs.insert(name.clone(), default_config);

        {
            use vil_log::app_log;
            app_log!(Info, "plugin.installed", { plugin: name.as_str() });
        }
        Ok(())
    }

    /// Enable a plugin.
    pub fn enable(&self, name: &str) -> Result<(), String> {
        let mut state = self
            .states
            .get_mut(name)
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        state.state = PluginState::Enabled;
        state.enabled_at = Some(timestamp);
        state.error_message = None;

        self.persist_state(name, &state)?;

        {
            use vil_log::app_log;
            app_log!(Info, "plugin.enabled", { plugin: name });
        }
        Ok(())
    }

    /// Disable a plugin.
    pub fn disable(&self, name: &str) -> Result<(), String> {
        let mut state = self
            .states
            .get_mut(name)
            .ok_or_else(|| format!("Plugin '{}' not found", name))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        state.state = PluginState::Disabled;
        state.disabled_at = Some(timestamp);

        self.persist_state(name, &state)?;

        {
            use vil_log::app_log;
            app_log!(Info, "plugin.disabled", { plugin: name });
        }
        Ok(())
    }

    /// Update plugin configuration (hot-reload).
    pub fn update_config(
        &self,
        name: &str,
        new_config: serde_json::Value,
    ) -> Result<Vec<String>, String> {
        let old_config = self
            .configs
            .get(name)
            .map(|c| c.clone())
            .unwrap_or(serde_json::Value::Null);

        // Detect changes
        let changes = diff_configs(&old_config, &new_config);

        // Encrypt secret fields before persisting
        let manifest = self.manifests.get(name);
        let config_to_persist = if let Some(m) = &manifest {
            self.encrypt_secrets(&new_config, &m.config_schema)?
        } else {
            new_config.clone()
        };

        // Write to disk
        let plugin_dir = self.plugins_dir.join(name);
        let config_yaml = serde_yaml::to_string(&config_to_persist)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(plugin_dir.join("config.yaml"), &config_yaml)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Update in-memory
        self.configs.insert(name.to_string(), new_config);

        // Increment config version
        if let Some(mut state) = self.states.get_mut(name) {
            state.config_version += 1;
            let _ = self.persist_state(name, &state);
        }

        {
            use vil_log::app_log;
            app_log!(Info, "plugin.config.updated", { plugin: name, changes: changes.len() as u64 });
        }
        Ok(changes)
    }

    /// Get plugin config (with secrets decrypted).
    pub fn get_config(&self, name: &str) -> Option<serde_json::Value> {
        self.configs.get(name).map(|c| c.clone())
    }

    /// Get plugin config (with secrets masked for API response).
    pub fn get_config_masked(&self, name: &str) -> Option<serde_json::Value> {
        let config = self.configs.get(name)?;
        let manifest = self.manifests.get(name)?;

        let mut masked = config.clone();
        if let Some(obj) = masked.as_object_mut() {
            for (key, field) in &manifest.config_schema {
                if field.secret {
                    if obj.contains_key(key) {
                        obj.insert(key.clone(), serde_json::json!("********"));
                    }
                }
            }
        }
        Some(masked)
    }

    /// Get plugin manifest.
    pub fn get_manifest(&self, name: &str) -> Option<PluginManifest> {
        self.manifests.get(name).map(|m| m.clone())
    }

    /// Get plugin state.
    pub fn get_state(&self, name: &str) -> Option<PluginRuntimeState> {
        self.states.get(name).map(|s| s.clone())
    }

    /// Check if plugin is enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.states
            .get(name)
            .map(|s| s.state == PluginState::Enabled)
            .unwrap_or(false)
    }

    /// List all installed plugins with their state.
    pub fn list_plugins(&self) -> Vec<PluginSummary> {
        let registry = self.registry.read().unwrap();
        registry
            .plugins
            .values()
            .map(|entry| {
                let state = self
                    .states
                    .get(&entry.name)
                    .map(|s| s.state.clone())
                    .unwrap_or(PluginState::Installed);
                let manifest = self.manifests.get(&entry.name);

                PluginSummary {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    plugin_type: entry.plugin_type.clone(),
                    tier: entry.tier.clone(),
                    state,
                    description: manifest
                        .as_ref()
                        .map(|m| m.description.clone())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Remove a plugin.
    pub fn remove(&self, name: &str) -> Result<(), String> {
        self.states.remove(name);
        self.manifests.remove(name);
        self.configs.remove(name);

        {
            let mut registry = self.registry.write().unwrap();
            registry.unregister(name);
            let _ = registry.save(&self.plugins_dir.join("registry.json"));
        }

        let plugin_dir = self.plugins_dir.join(name);
        if plugin_dir.exists() {
            let _ = std::fs::remove_dir_all(&plugin_dir);
        }

        {
            use vil_log::app_log;
            app_log!(Info, "plugin.removed", { plugin: name });
        }
        Ok(())
    }

    /// Get plugin count.
    pub fn plugin_count(&self) -> usize {
        self.registry.read().unwrap().count()
    }

    // --- Internal helpers ---

    fn build_default_config(&self, manifest: &PluginManifest) -> serde_json::Value {
        let mut config = serde_json::Map::new();
        for (key, field) in &manifest.config_schema {
            if let Some(default) = &field.default {
                config.insert(key.clone(), default.clone());
            }
        }
        serde_json::Value::Object(config)
    }

    fn encrypt_secrets(
        &self,
        config: &serde_json::Value,
        schema: &std::collections::HashMap<String, ConfigField>,
    ) -> Result<serde_json::Value, String> {
        let mut result = config.clone();
        if let Some(obj) = result.as_object_mut() {
            for (key, field) in schema {
                if field.secret {
                    if let Some(val) = obj.get(key) {
                        if let Some(s) = val.as_str() {
                            if !s.starts_with("ENC[") && !s.starts_with("${") {
                                let encrypted = self
                                    .secrets
                                    .encrypt(s)
                                    .map_err(|e| format!("Encryption failed for {}: {}", key, e))?;
                                obj.insert(key.clone(), serde_json::json!(encrypted));
                            }
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    fn persist_state(&self, name: &str, state: &PluginRuntimeState) -> Result<(), String> {
        let plugin_dir = self.plugins_dir.join(name);
        let state_json = serde_json::to_string_pretty(&*state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        std::fs::write(plugin_dir.join("state.json"), &state_json)
            .map_err(|e| format!("Failed to write state: {}", e))?;
        Ok(())
    }
}

/// Plugin summary for list endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginSummary {
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub tier: PluginTier,
    pub state: PluginState,
    pub description: String,
}

/// Diff two config values and return list of changes.
fn diff_configs(old: &serde_json::Value, new: &serde_json::Value) -> Vec<String> {
    let mut changes = Vec::new();
    if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
        for (key, new_val) in new_obj {
            match old_obj.get(key) {
                Some(old_val) if old_val != new_val => {
                    changes.push(format!("{}: {} → {}", key, old_val, new_val));
                }
                None => {
                    changes.push(format!("{}: (new) {}", key, new_val));
                }
                _ => {}
            }
        }
    }
    changes
}
