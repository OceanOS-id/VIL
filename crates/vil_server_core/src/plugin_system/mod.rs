//! VIL Hybrid Plugin System
//!
//! Four-tier plugin architecture:
//!   - Tier 1: Native (compile-time trait, zero overhead)
//!   - Tier 2: Process (ServiceProcess via Tri-Lane, ~50ns)
//!   - Tier 3: WASM (sandboxed, hot-deploy, ~1-5us)
//!   - Tier 4: Sidecar (any language via UDS, ~12us)

pub mod resource;
pub mod context;
pub mod registry;
pub mod semantic;

pub use resource::ResourceRegistry;
pub use context::PluginContext;
pub use registry::{PluginRegistry, PluginError, PluginInfo};
pub use semantic::{AiSemantic, AiSemanticKind, AiLane, AiSemanticEnvelope};

/// Core plugin trait -- all VIL plugins implement this.
///
/// Plugins are registered via `VilApp::new("app").plugin(MyPlugin::new())`.
/// The plugin system resolves dependencies and initializes plugins in order.
pub trait VilPlugin: Send + Sync + 'static {
    /// Unique plugin identifier (e.g., "vil-llm", "vil-rag")
    fn id(&self) -> &str;

    /// Semantic version (e.g., "1.0.0")
    fn version(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str { "" }

    /// What this plugin provides
    fn capabilities(&self) -> Vec<PluginCapability> { vec![] }

    /// What this plugin requires from other plugins
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    /// Register the plugin -- add services, resources, middleware.
    /// Called in dependency order (dependencies registered first).
    fn register(&self, ctx: &mut PluginContext);

    /// Health check (called periodically by observer)
    fn health(&self) -> PluginHealth { PluginHealth::Healthy }

    /// Graceful shutdown hook
    fn shutdown(&self) {}
}

/// What a plugin can provide
#[derive(Debug, Clone)]
pub enum PluginCapability {
    /// Adds a ServiceProcess with endpoints
    Service {
        name: String,
        endpoints: Vec<EndpointSpec>,
    },
    /// Adds middleware layer (priority: lower = runs first)
    Middleware {
        name: String,
        priority: u32,
    },
    /// Adds CLI subcommand
    CliCommand {
        name: String,
        description: String,
    },
    /// Provides a typed resource other plugins can consume
    Resource {
        type_name: &'static str,
        name: String,
    },
    /// Extends observer dashboard
    DashboardWidget {
        name: String,
    },
}

/// Endpoint specification for capability declaration
#[derive(Debug, Clone)]
pub struct EndpointSpec {
    pub method: String,
    pub path: String,
    pub description: String,
}

impl EndpointSpec {
    pub fn get(path: &str) -> Self {
        Self { method: "GET".into(), path: path.into(), description: String::new() }
    }
    pub fn post(path: &str) -> Self {
        Self { method: "POST".into(), path: path.into(), description: String::new() }
    }
    pub fn delete(path: &str) -> Self {
        Self { method: "DELETE".into(), path: path.into(), description: String::new() }
    }
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.into();
        self
    }
}

/// Plugin dependency declaration
#[derive(Debug, Clone)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_req: String,
    pub optional: bool,
}

impl PluginDependency {
    pub fn required(id: &str, version: &str) -> Self {
        Self { plugin_id: id.into(), version_req: version.into(), optional: false }
    }
    pub fn optional(id: &str, version: &str) -> Self {
        Self { plugin_id: id.into(), version_req: version.into(), optional: true }
    }
}

/// Plugin health status
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum PluginHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
