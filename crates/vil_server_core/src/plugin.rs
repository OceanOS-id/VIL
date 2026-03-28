// =============================================================================
// VIL Server — Native Plugin System
// =============================================================================
//
// Dynamic loading of native plugins (.so/.dylib) at runtime.
// Plugins implement the VilPlugin trait and are loaded via libloading.
//
// Safety: Native plugins have full access to the process.
// For sandboxed execution, use WASM capsules instead.
//
// Plugin lifecycle:
//   1. Load: dlopen the shared library
//   2. Init: call plugin_init() export
//   3. Route: plugin registers routes via callback
//   4. Unload: call plugin_shutdown(), then dlclose

use std::collections::HashMap;

/// Plugin metadata returned by the plugin's init function.
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Routes the plugin wants to register
    pub routes: Vec<PluginRoute>,
    /// Required capabilities
    pub capabilities: Vec<String>,
}

/// A route registered by a plugin.
#[derive(Debug, Clone)]
pub struct PluginRoute {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Path pattern
    pub path: String,
    /// Handler identifier within the plugin
    pub handler_id: String,
}

/// Plugin status.
#[derive(Debug, Clone, serde::Serialize)]
pub enum PluginStatus {
    Loaded,
    Active,
    Error(String),
    Unloaded,
}

/// Plugin registry — manages loaded plugins.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginInfo>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PluginInfo {
    manifest: PluginManifest,
    status: PluginStatus,
    load_path: String,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin manifest (called after loading).
    pub fn register(&mut self, manifest: PluginManifest, load_path: &str) {
        let name = manifest.name.clone();
        {
            use vil_log::app_log;
            app_log!(Info, "plugin.registered", { plugin: name.as_str(), routes: manifest.routes.len() as u64 });
        }
        self.plugins.insert(name, PluginInfo {
            manifest,
            status: PluginStatus::Active,
            load_path: load_path.to_string(),
        });
    }

    /// Unregister a plugin.
    pub fn unregister(&mut self, name: &str) {
        if let Some(info) = self.plugins.get_mut(name) {
            info.status = PluginStatus::Unloaded;
            {
                use vil_log::app_log;
                app_log!(Info, "plugin.unregistered", { plugin: name });
            }
        }
    }

    /// Get plugin status.
    pub fn status(&self, name: &str) -> Option<&PluginStatus> {
        self.plugins.get(name).map(|p| &p.status)
    }

    /// List all registered plugins.
    pub fn list(&self) -> Vec<PluginSummary> {
        self.plugins.iter().map(|(name, info)| PluginSummary {
            name: name.clone(),
            version: info.manifest.version.clone(),
            description: info.manifest.description.clone(),
            routes: info.manifest.routes.len(),
            status: info.status.clone(),
        }).collect()
    }

    /// Get plugin count.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin summary for admin endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub routes: usize,
    pub status: PluginStatus,
}
