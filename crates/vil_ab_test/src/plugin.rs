//! VilPlugin implementation for A/B testing integration.

use vil_server::prelude::*;
use std::sync::Arc;

use crate::handlers::{self, ExperimentRegistry};
use crate::semantic::{AbTestEvent, AbTestFault, AbTestState};

/// A/B test plugin — experiment management with statistical significance.
pub struct AbTestPlugin {
    registry: Arc<ExperimentRegistry>,
}

impl AbTestPlugin {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ExperimentRegistry::new()),
        }
    }

    /// Create with a pre-populated registry.
    pub fn with_registry(registry: Arc<ExperimentRegistry>) -> Self {
        Self { registry }
    }

    /// Access the shared experiment registry.
    pub fn registry(&self) -> &Arc<ExperimentRegistry> {
        &self.registry
    }
}

impl Default for AbTestPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for AbTestPlugin {
    fn id(&self) -> &str {
        "vil-ab-test"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "A/B testing for AI models and prompts with statistical significance"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "abtest".into(),
            endpoints: vec![
                EndpointSpec::get("/api/abtest/stats").with_description("A/B test stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    fn register(&self, ctx: &mut PluginContext) {
        let registry = Arc::clone(&self.registry);

        let svc = ServiceProcess::new("ab-test")
            .state(registry)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<AbTestEvent>()
            .faults::<AbTestFault>()
            .manages::<AbTestState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
