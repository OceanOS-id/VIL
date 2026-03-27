//! VilPlugin implementation for Model Serving.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::semantic::{ServingEvent, ServingFault, ServingState};
use crate::serving::ModelServer;

pub struct ModelServingPlugin {
    server: Arc<ModelServer>,
}

impl ModelServingPlugin {
    pub fn new(server: Arc<ModelServer>) -> Self {
        Self { server }
    }
}

impl VilPlugin for ModelServingPlugin {
    fn id(&self) -> &str { "vil-model-serving" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Model lifecycle management with canary deployment" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "model-serving".into(),
            endpoints: vec![
                EndpointSpec::post("/api/serving/infer"),
                EndpointSpec::get("/api/serving/models"),
                EndpointSpec::get("/api/serving/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<Arc<ModelServer>>("model-server", self.server.clone());

        let svc = ServiceProcess::new("model-serving")
            .state(self.server.clone())
            .emits::<ServingEvent>()
            .faults::<ServingFault>()
            .manages::<ServingState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
