//! VilPlugin implementation for AI Gateway.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::gateway::AiGateway;
use crate::semantic::{GatewayEvent, GatewayFault, GatewayState};

pub struct AiGatewayPlugin {
    gateway: Arc<AiGateway>,
}

impl AiGatewayPlugin {
    pub fn new(gateway: Arc<AiGateway>) -> Self {
        Self { gateway }
    }
}

impl VilPlugin for AiGatewayPlugin {
    fn id(&self) -> &str { "vil-ai-gateway" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Unified AI API gateway with circuit breaker and cost tracking" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "ai-gateway".into(),
            endpoints: vec![
                EndpointSpec::post("/api/gateway/chat"),
                EndpointSpec::get("/api/gateway/stats"),
                EndpointSpec::get("/api/gateway/health"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<Arc<AiGateway>>("ai-gateway", self.gateway.clone());

        let svc = ServiceProcess::new("ai-gateway")
            .state(self.gateway.clone())
            .emits::<GatewayEvent>()
            .faults::<GatewayFault>()
            .manages::<GatewayState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
