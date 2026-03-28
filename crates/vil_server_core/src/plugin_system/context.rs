//! PluginContext -- passed to plugins during registration.

use crate::vx::service::ServiceProcess;
use super::resource::ResourceRegistry;

/// Context passed to VilPlugin::register().
///
/// Provides access to:
/// - Add services to the app
/// - Provide/require typed resources
/// - Add mesh routes between services
pub struct PluginContext<'a> {
    /// Services to add to the app
    services: &'a mut Vec<ServiceProcess>,
    /// Typed resource registry (shared across all plugins)
    resources: &'a mut ResourceRegistry,
    /// Mesh routes to add: (from_service, to_service)
    mesh_routes: &'a mut Vec<(String, String)>,
    /// Current plugin ID (for error messages)
    plugin_id: String,
}

impl<'a> PluginContext<'a> {
    pub fn new(
        plugin_id: &str,
        services: &'a mut Vec<ServiceProcess>,
        resources: &'a mut ResourceRegistry,
        mesh_routes: &'a mut Vec<(String, String)>,
    ) -> Self {
        Self {
            services,
            resources,
            mesh_routes,
            plugin_id: plugin_id.to_string(),
        }
    }

    /// Add a ServiceProcess to the app.
    pub fn add_service(&mut self, svc: ServiceProcess) {
        {
            use vil_log::app_log;
            app_log!(Info, "plugin.service.registered", { plugin: self.plugin_id.as_str(), service: svc.name() });
        }
        self.services.push(svc);
    }

    /// Register a typed resource (other plugins can consume it).
    pub fn provide<T: Send + Sync + 'static>(&mut self, name: &str, resource: T) {
        // debug-level: skip vil_log
        self.resources.provide::<T>(name, resource);
    }

    /// Get a resource provided by another plugin.
    /// Returns None if not found (plugin not registered yet or wrong type).
    pub fn get<T: Send + Sync + 'static>(&self, name: &str) -> Option<&T> {
        self.resources.get::<T>(name)
    }

    /// Get a resource, panic if not found (use when dependency is required).
    pub fn require<T: Send + Sync + 'static>(&self, name: &str) -> &T {
        self.resources.require::<T>(name)
    }

    /// Check if a resource exists.
    pub fn has_resource<T: Send + Sync + 'static>(&self, name: &str) -> bool {
        self.resources.has::<T>(name)
    }

    /// Add a mesh route between two services.
    pub fn add_route(&mut self, from: &str, to: &str) {
        self.mesh_routes.push((from.to_string(), to.to_string()));
    }

    /// Get the current plugin ID.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Get read-only access to the resource registry.
    pub fn resources(&self) -> &ResourceRegistry {
        self.resources
    }
}
