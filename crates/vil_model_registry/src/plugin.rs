//! VilPlugin implementation for model registry integration.

use vil_server::prelude::*;

use std::sync::Arc;

use crate::handlers;
use crate::registry::ModelRegistry;
use crate::semantic::{RegistryEvent, RegistryFault, RegistryState};

/// Model registry plugin — versioned model artifact management.
pub struct ModelRegistryPlugin {
    registry: Arc<ModelRegistry>,
}

impl ModelRegistryPlugin {
    pub fn new(registry: Arc<ModelRegistry>) -> Self {
        Self { registry }
    }
}

impl Default for ModelRegistryPlugin {
    fn default() -> Self {
        Self {
            registry: Arc::new(ModelRegistry::new()),
        }
    }
}

impl VilPlugin for ModelRegistryPlugin {
    fn id(&self) -> &str {
        "vil-model-registry"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Versioned model artifact management with promotion and rollback"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "registry".into(),
            endpoints: vec![
                EndpointSpec::get("/api/registry/models")
                    .with_description("List registered models"),
                EndpointSpec::get("/api/registry/stats").with_description("Registry stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("model-registry")
            .state(Arc::clone(&self.registry))
            .endpoint(Method::GET, "/models", get(handlers::models_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<RegistryEvent>()
            .faults::<RegistryFault>()
            .manages::<RegistryState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
