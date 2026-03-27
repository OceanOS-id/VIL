//! PluginRegistry -- manages plugin lifecycle and dependency resolution.

use super::{VilPlugin, PluginHealth, PluginCapability, PluginContext};
use super::resource::ResourceRegistry;
use crate::vx::service::ServiceProcess;
use std::collections::HashMap;

/// Information about a registered plugin.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub health: String,
}

/// Plugin registration and lifecycle errors.
#[derive(Debug)]
pub enum PluginError {
    /// Circular dependency detected
    CircularDependency(Vec<String>),
    /// Required dependency not registered
    MissingDependency { plugin: String, requires: String },
    /// Plugin registration failed
    RegistrationFailed { plugin: String, error: String },
    /// Duplicate plugin ID
    DuplicatePlugin(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircularDependency(cycle) => write!(f, "circular plugin dependency: {}", cycle.join(" -> ")),
            Self::MissingDependency { plugin, requires } => write!(f, "plugin '{}' requires '{}' which is not registered", plugin, requires),
            Self::RegistrationFailed { plugin, error } => write!(f, "plugin '{}' registration failed: {}", plugin, error),
            Self::DuplicatePlugin(id) => write!(f, "duplicate plugin ID: '{}'", id),
        }
    }
}

impl std::error::Error for PluginError {}

/// Manages plugin registration, dependency resolution, and lifecycle.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn VilPlugin>>,
    resources: ResourceRegistry,
    resolved: bool,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            resources: ResourceRegistry::new(),
            resolved: false,
        }
    }

    /// Add a plugin to the registry.
    pub fn add(&mut self, plugin: impl VilPlugin) {
        self.plugins.push(Box::new(plugin));
        self.resolved = false;
    }

    /// Number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Resolve dependencies and register all plugins in order.
    /// Returns (services_to_add, mesh_routes_to_add).
    pub fn resolve_and_register(&mut self) -> Result<(Vec<ServiceProcess>, Vec<(String, String)>), PluginError> {
        if self.plugins.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        // Check for duplicates
        let mut seen = HashMap::new();
        for (idx, plugin) in self.plugins.iter().enumerate() {
            if let Some(_prev_idx) = seen.insert(plugin.id().to_string(), idx) {
                return Err(PluginError::DuplicatePlugin(plugin.id().to_string()));
            }
        }

        // Resolve dependency order (topological sort)
        let order = self.topological_sort()?;

        // Register in order
        let mut services = Vec::new();
        let mut mesh_routes = Vec::new();

        for idx in order {
            let plugin = &self.plugins[idx];
            let plugin_id = plugin.id().to_string();

            tracing::info!(
                plugin = %plugin_id,
                version = %plugin.version(),
                "registering plugin"
            );

            let mut ctx = PluginContext::new(
                &plugin_id,
                &mut services,
                &mut self.resources,
                &mut mesh_routes,
            );

            plugin.register(&mut ctx);
        }

        self.resolved = true;
        Ok((services, mesh_routes))
    }

    /// Topological sort using Kahn's algorithm.
    fn topological_sort(&self) -> Result<Vec<usize>, PluginError> {
        let n = self.plugins.len();

        // Build name -> index map
        let id_to_idx: HashMap<String, usize> = self.plugins.iter()
            .enumerate()
            .map(|(i, p)| (p.id().to_string(), i))
            .collect();

        // Build adjacency list and in-degree count
        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        for (idx, plugin) in self.plugins.iter().enumerate() {
            for dep in plugin.dependencies() {
                if let Some(&dep_idx) = id_to_idx.get(&dep.plugin_id) {
                    adj[dep_idx].push(idx); // dep_idx must come before idx
                    in_degree[idx] += 1;
                } else if !dep.optional {
                    return Err(PluginError::MissingDependency {
                        plugin: plugin.id().to_string(),
                        requires: dep.plugin_id.clone(),
                    });
                }
                // Optional missing dependency: skip silently
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut result = Vec::with_capacity(n);

        while let Some(node) = queue.pop() {
            result.push(node);
            for &next in &adj[node] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push(next);
                }
            }
        }

        if result.len() != n {
            // Find cycle
            let cycle_nodes: Vec<String> = (0..n)
                .filter(|&i| in_degree[i] > 0)
                .map(|i| self.plugins[i].id().to_string())
                .collect();
            return Err(PluginError::CircularDependency(cycle_nodes));
        }

        Ok(result)
    }

    /// List all registered plugins.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|p| {
            let caps: Vec<String> = p.capabilities().iter().map(|c| match c {
                PluginCapability::Service { name, .. } => format!("service:{}", name),
                PluginCapability::Middleware { name, .. } => format!("middleware:{}", name),
                PluginCapability::CliCommand { name, .. } => format!("cli:{}", name),
                PluginCapability::Resource { type_name, name } => format!("resource:{}({})", type_name, name),
                PluginCapability::DashboardWidget { name } => format!("widget:{}", name),
            }).collect();

            let health = match p.health() {
                PluginHealth::Healthy => "healthy".into(),
                PluginHealth::Degraded(msg) => format!("degraded: {}", msg),
                PluginHealth::Unhealthy(msg) => format!("unhealthy: {}", msg),
            };

            PluginInfo {
                id: p.id().to_string(),
                version: p.version().to_string(),
                description: p.description().to_string(),
                capabilities: caps,
                health,
            }
        }).collect()
    }

    /// Get health of all plugins.
    pub fn health_all(&self) -> Vec<(String, PluginHealth)> {
        self.plugins.iter()
            .map(|p| (p.id().to_string(), p.health()))
            .collect()
    }

    /// Get the resource registry.
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    /// Shutdown all plugins in reverse order.
    pub fn shutdown_all(&self) {
        for plugin in self.plugins.iter().rev() {
            tracing::info!(plugin = %plugin.id(), "shutting down plugin");
            plugin.shutdown();
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_system::PluginDependency;

    // --- Test plugins ---

    struct PluginA;
    impl VilPlugin for PluginA {
        fn id(&self) -> &str { "plugin-a" }
        fn version(&self) -> &str { "1.0.0" }
        fn register(&self, ctx: &mut PluginContext) {
            ctx.provide::<String>("greeting", "hello from A".into());
        }
    }

    struct PluginB;
    impl VilPlugin for PluginB {
        fn id(&self) -> &str { "plugin-b" }
        fn version(&self) -> &str { "1.0.0" }
        fn dependencies(&self) -> Vec<PluginDependency> {
            vec![PluginDependency::required("plugin-a", ">=1.0")]
        }
        fn register(&self, ctx: &mut PluginContext) {
            let greeting = ctx.require::<String>("greeting");
            ctx.provide::<String>("extended", format!("{} + B", greeting));
        }
    }

    struct PluginC;
    impl VilPlugin for PluginC {
        fn id(&self) -> &str { "plugin-c" }
        fn version(&self) -> &str { "1.0.0" }
        fn dependencies(&self) -> Vec<PluginDependency> {
            vec![PluginDependency::required("plugin-b", ">=1.0")]
        }
        fn register(&self, ctx: &mut PluginContext) {
            let extended = ctx.require::<String>("extended");
            assert!(extended.contains("hello from A"));
            assert!(extended.contains("B"));
        }
    }

    #[test]
    fn test_linear_dependency_resolution() {
        let mut reg = PluginRegistry::new();
        // Register in WRONG order -- resolver should fix it
        reg.add(PluginC);
        reg.add(PluginA);
        reg.add(PluginB);

        let (services, _routes) = reg.resolve_and_register().unwrap();
        assert!(services.is_empty()); // these plugins don't add services
        assert_eq!(reg.resources().require::<String>("extended"), "hello from A + B");
    }

    #[test]
    fn test_missing_required_dependency() {
        let mut reg = PluginRegistry::new();
        reg.add(PluginB); // requires plugin-a, which is not registered

        let result = reg.resolve_and_register();
        assert!(matches!(result, Err(PluginError::MissingDependency { .. })));
    }

    #[test]
    fn test_circular_dependency() {
        struct CycleA;
        impl VilPlugin for CycleA {
            fn id(&self) -> &str { "cycle-a" }
            fn version(&self) -> &str { "1.0.0" }
            fn dependencies(&self) -> Vec<PluginDependency> {
                vec![PluginDependency::required("cycle-b", ">=1.0")]
            }
            fn register(&self, _ctx: &mut PluginContext) {}
        }
        struct CycleB;
        impl VilPlugin for CycleB {
            fn id(&self) -> &str { "cycle-b" }
            fn version(&self) -> &str { "1.0.0" }
            fn dependencies(&self) -> Vec<PluginDependency> {
                vec![PluginDependency::required("cycle-a", ">=1.0")]
            }
            fn register(&self, _ctx: &mut PluginContext) {}
        }

        let mut reg = PluginRegistry::new();
        reg.add(CycleA);
        reg.add(CycleB);

        let result = reg.resolve_and_register();
        assert!(matches!(result, Err(PluginError::CircularDependency(_))));
    }

    #[test]
    fn test_duplicate_plugin_id() {
        let mut reg = PluginRegistry::new();
        reg.add(PluginA);
        reg.add(PluginA);

        let result = reg.resolve_and_register();
        assert!(matches!(result, Err(PluginError::DuplicatePlugin(_))));
    }

    #[test]
    fn test_optional_dependency_missing_ok() {
        struct OptPlugin;
        impl VilPlugin for OptPlugin {
            fn id(&self) -> &str { "opt-plugin" }
            fn version(&self) -> &str { "1.0.0" }
            fn dependencies(&self) -> Vec<PluginDependency> {
                vec![PluginDependency::optional("nonexistent", ">=1.0")]
            }
            fn register(&self, ctx: &mut PluginContext) {
                ctx.provide::<String>("opt-result", "ok".into());
            }
        }

        let mut reg = PluginRegistry::new();
        reg.add(OptPlugin);
        let (_, _) = reg.resolve_and_register().unwrap();
        assert_eq!(reg.resources().require::<String>("opt-result"), "ok");
    }

    #[test]
    fn test_plugin_list() {
        let mut reg = PluginRegistry::new();
        reg.add(PluginA);
        reg.add(PluginB);
        let list = reg.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "plugin-a");
    }

    #[test]
    fn test_empty_registry() {
        let mut reg = PluginRegistry::new();
        let (services, routes) = reg.resolve_and_register().unwrap();
        assert!(services.is_empty());
        assert!(routes.is_empty());
    }

    #[test]
    fn test_plugin_adds_service() {
        use crate::vx::service::ServiceProcess;
        use crate::plugin_system::EndpointSpec;

        struct SvcPlugin;
        impl VilPlugin for SvcPlugin {
            fn id(&self) -> &str { "svc-plugin" }
            fn version(&self) -> &str { "1.0.0" }
            fn capabilities(&self) -> Vec<PluginCapability> {
                vec![PluginCapability::Service {
                    name: "my-svc".into(),
                    endpoints: vec![EndpointSpec::get("/api/hello")],
                }]
            }
            fn register(&self, ctx: &mut PluginContext) {
                ctx.add_service(ServiceProcess::new("my-svc"));
            }
        }

        let mut reg = PluginRegistry::new();
        reg.add(SvcPlugin);
        let (services, _) = reg.resolve_and_register().unwrap();
        assert_eq!(services.len(), 1);
    }
}
