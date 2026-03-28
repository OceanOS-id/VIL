use vil_server::prelude::*;

use crate::fusion::FusionEngine;
use crate::handlers;
use crate::vil_semantic::{MultimodalEvent, MultimodalFault, MultimodalState};
use std::sync::Arc;

pub struct MultimodalPlugin;

impl MultimodalPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MultimodalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for MultimodalPlugin {
    fn id(&self) -> &str {
        "vil-multimodal"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Text + image + audio fusion with cross-modality search"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "multimodal".into(),
            endpoints: vec![EndpointSpec::get("/api/multimodal/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            plugin_id: "vil-embedder".into(),
            version_req: ">=0.1.0".into(),
            optional: true,
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let engine = Arc::new(FusionEngine::new());

        let svc = ServiceProcess::new("multimodal")
            .state(engine)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<MultimodalEvent>()
            .faults::<MultimodalFault>()
            .manages::<MultimodalState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
