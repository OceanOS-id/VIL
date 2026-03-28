//! VilPlugin implementation for Multi-Agent.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::orchestrator::Orchestrator;
use crate::semantic::{MultiAgentEvent, MultiAgentFault, MultiAgentState};

pub struct MultiAgentPlugin {
    orchestrator: Arc<tokio::sync::Mutex<Orchestrator>>,
}

impl MultiAgentPlugin {
    pub fn new(orchestrator: Orchestrator) -> Self {
        Self {
            orchestrator: Arc::new(tokio::sync::Mutex::new(orchestrator)),
        }
    }
}

impl VilPlugin for MultiAgentPlugin {
    fn id(&self) -> &str {
        "vil-multi-agent"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Orchestrated multi-agent collaboration"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "multi-agent".into(),
            endpoints: vec![
                EndpointSpec::post("/api/multi-agent/run"),
                EndpointSpec::get("/api/multi-agent/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<Arc<tokio::sync::Mutex<Orchestrator>>>(
            "multi-agent",
            self.orchestrator.clone(),
        );

        let svc = ServiceProcess::new("multi-agent")
            .state(self.orchestrator.clone())
            .emits::<MultiAgentEvent>()
            .faults::<MultiAgentFault>()
            .manages::<MultiAgentState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
