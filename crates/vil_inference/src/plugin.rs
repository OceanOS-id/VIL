use vil_server::prelude::*;

use std::sync::Arc;
use crate::registry::ModelRegistry;
use crate::handlers;
use crate::semantic::{InferEvent, InferFault, InferState};

pub struct InferencePlugin;

impl InferencePlugin {
    pub fn new() -> Self { Self }
}

impl Default for InferencePlugin {
    fn default() -> Self { Self::new() }
}

impl VilPlugin for InferencePlugin {
    fn id(&self) -> &str { "vil-inference" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Model pool, dynamic batching, hot-swap inference" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "inference".into(),
            endpoints: vec![EndpointSpec::get("/api/inference/models")],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let registry = Arc::new(ModelRegistry::new());
        ctx.provide::<Arc<ModelRegistry>>("model-registry", registry.clone());

        let svc = ServiceProcess::new("inference")
            .endpoint(Method::GET, "/models", get(handlers::list_models_handler))
            .state(registry)
            .emits::<InferEvent>()
            .faults::<InferFault>()
            .manages::<InferState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
