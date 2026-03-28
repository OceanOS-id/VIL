use vil_server::prelude::*;

use crate::config::SpeculativeConfig;
use crate::handlers;
use crate::semantic::{SpeculativeEvent, SpeculativeFault, SpeculativeState};
use std::sync::Arc;

pub struct SpeculativePlugin;

impl SpeculativePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl VilPlugin for SpeculativePlugin {
    fn id(&self) -> &str {
        "vil-speculative"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Speculative decoding proxy for 2-3x faster LLM generation"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "speculative".into(),
            endpoints: vec![EndpointSpec::get("/api/speculative/stats")],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let config = Arc::new(SpeculativeConfig::default());

        let svc = ServiceProcess::new("speculative")
            .state(config)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<SpeculativeEvent>()
            .faults::<SpeculativeFault>()
            .manages::<SpeculativeState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
