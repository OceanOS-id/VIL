// =============================================================================
// Plugin Testing Utilities — Unit test plugins without starting a server
// =============================================================================
//
// Example:
//   use vil_plugin_sdk::testing::PluginTestHarness;
//
//   #[test]
//   fn test_my_plugin() {
//       let mut harness = PluginTestHarness::new();
//       let plugin = MyPlugin::new();
//
//       // Register and inspect results
//       harness.register(&plugin);
//
//       assert_eq!(harness.service_count(), 1);
//       assert!(harness.has_resource::<MyState>("my-state"));
//       assert_eq!(harness.route_count(), 0);
//   }

use crate::{VilPlugin, PluginContext, ResourceRegistry, ServiceProcess};

/// Test harness for unit testing VilPlugin implementations.
///
/// Provides a mock PluginContext that captures services, resources,
/// and mesh routes without starting a real server.
pub struct PluginTestHarness {
    services: Vec<ServiceProcess>,
    resources: ResourceRegistry,
    routes: Vec<(String, String)>,
}

impl PluginTestHarness {
    /// Create a new test harness.
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            resources: ResourceRegistry::new(),
            routes: Vec::new(),
        }
    }

    /// Register a plugin into the test harness.
    pub fn register(&mut self, plugin: &dyn VilPlugin) {
        let mut ctx = PluginContext::new(
            plugin.id(),
            &mut self.services,
            &mut self.resources,
            &mut self.routes,
        );
        plugin.register(&mut ctx);
    }

    /// Register multiple plugins in order (no dependency resolution).
    pub fn register_all(&mut self, plugins: &[&dyn VilPlugin]) {
        for plugin in plugins {
            self.register(*plugin);
        }
    }

    // ── Inspection ──

    /// Number of services added by plugins.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Get service names.
    pub fn service_names(&self) -> Vec<&str> {
        self.services.iter().map(|s| s.name()).collect()
    }

    /// Check if a resource was provided.
    pub fn has_resource<T: Send + Sync + 'static>(&self, name: &str) -> bool {
        self.resources.has::<T>(name)
    }

    /// Get a resource by type and name.
    pub fn get_resource<T: Send + Sync + 'static>(&self, name: &str) -> Option<&T> {
        self.resources.get::<T>(name)
    }

    /// Number of mesh routes added.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Get mesh routes as (from, to) pairs.
    pub fn routes(&self) -> &[(String, String)] {
        &self.routes
    }

    /// Total resources registered.
    pub fn resource_count(&self) -> usize {
        self.resources.count()
    }

    /// Get the resource registry for advanced inspection.
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }
}

impl Default for PluginTestHarness {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginCapability, EndpointSpec, PluginHealth};

    struct TestPlugin;

    impl VilPlugin for TestPlugin {
        fn id(&self) -> &str { "test-plugin" }
        fn version(&self) -> &str { "1.0.0" }
        fn description(&self) -> &str { "Test plugin for harness" }

        fn capabilities(&self) -> Vec<PluginCapability> {
            vec![PluginCapability::Service {
                name: "test-svc".into(),
                endpoints: vec![EndpointSpec::get("/test")],
            }]
        }

        fn register(&self, ctx: &mut PluginContext) {
            ctx.provide::<String>("greeting", "hello from test".into());
            ctx.add_service(ServiceProcess::new("test-svc"));
        }

        fn health(&self) -> PluginHealth {
            PluginHealth::Healthy
        }
    }

    #[test]
    fn test_harness_captures_services() {
        let mut harness = PluginTestHarness::new();
        harness.register(&TestPlugin);

        assert_eq!(harness.service_count(), 1);
        assert_eq!(harness.service_names(), vec!["test-svc"]);
    }

    #[test]
    fn test_harness_captures_resources() {
        let mut harness = PluginTestHarness::new();
        harness.register(&TestPlugin);

        assert!(harness.has_resource::<String>("greeting"));
        assert_eq!(
            harness.get_resource::<String>("greeting"),
            Some(&"hello from test".to_string())
        );
    }

    #[test]
    fn test_harness_empty() {
        let harness = PluginTestHarness::new();
        assert_eq!(harness.service_count(), 0);
        assert_eq!(harness.route_count(), 0);
        assert_eq!(harness.resource_count(), 0);
    }
}
