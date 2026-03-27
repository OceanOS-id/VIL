// =============================================================================
// PluginBuilder — Ergonomic plugin construction
// =============================================================================
//
// Provides a builder pattern alternative to implementing VilPlugin directly.
// Useful for simple plugins that don't need a custom struct.
//
// Example:
//   let plugin = PluginBuilder::new("my-plugin", "1.0.0")
//       .description("My awesome plugin")
//       .capability(PluginCapability::Service { ... })
//       .dependency(PluginDependency::required("vil-llm", ">=0.1"))
//       .on_register(|ctx| {
//           ctx.add_service(ServiceProcess::new("my-svc"));
//       })
//       .build();
//
//   VilApp::new("app").plugin(plugin).run().await;

use crate::{
    VilPlugin, PluginContext, PluginCapability,
    PluginDependency, PluginHealth,
};

type RegisterFn = Box<dyn Fn(&mut PluginContext) + Send + Sync + 'static>;
type HealthFn = Box<dyn Fn() -> PluginHealth + Send + Sync + 'static>;

/// Ergonomic builder for creating VilPlugin implementations.
pub struct PluginBuilder {
    id: String,
    version: String,
    description: String,
    capabilities: Vec<PluginCapability>,
    dependencies: Vec<PluginDependency>,
    register_fn: Option<RegisterFn>,
    health_fn: Option<HealthFn>,
}

impl PluginBuilder {
    /// Create a new plugin builder with required id and version.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            description: String::new(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            register_fn: None,
            health_fn: None,
        }
    }

    /// Set human-readable description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a capability declaration.
    pub fn capability(mut self, cap: PluginCapability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add a dependency.
    pub fn dependency(mut self, dep: PluginDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set the registration callback (called during plugin initialization).
    pub fn on_register(mut self, f: impl Fn(&mut PluginContext) + Send + Sync + 'static) -> Self {
        self.register_fn = Some(Box::new(f));
        self
    }

    /// Set a custom health check function.
    pub fn on_health(mut self, f: impl Fn() -> PluginHealth + Send + Sync + 'static) -> Self {
        self.health_fn = Some(Box::new(f));
        self
    }

    /// Build the plugin. Consumes the builder.
    pub fn build(self) -> BuiltPlugin {
        BuiltPlugin {
            id: self.id,
            version: self.version,
            description: self.description,
            capabilities: self.capabilities,
            dependencies: self.dependencies,
            register_fn: self.register_fn.unwrap_or_else(|| Box::new(|_| {})),
            health_fn: self.health_fn,
        }
    }
}

/// A plugin created via PluginBuilder.
pub struct BuiltPlugin {
    id: String,
    version: String,
    description: String,
    capabilities: Vec<PluginCapability>,
    dependencies: Vec<PluginDependency>,
    register_fn: RegisterFn,
    health_fn: Option<HealthFn>,
}

impl VilPlugin for BuiltPlugin {
    fn id(&self) -> &str { &self.id }
    fn version(&self) -> &str { &self.version }
    fn description(&self) -> &str { &self.description }
    fn capabilities(&self) -> Vec<PluginCapability> { self.capabilities.clone() }
    fn dependencies(&self) -> Vec<PluginDependency> { self.dependencies.clone() }

    fn register(&self, ctx: &mut PluginContext) {
        (self.register_fn)(ctx);
    }

    fn health(&self) -> PluginHealth {
        match &self.health_fn {
            Some(f) => f(),
            None => PluginHealth::Healthy,
        }
    }
}
