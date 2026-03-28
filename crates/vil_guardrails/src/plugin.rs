use vil_server::prelude::*;

use crate::handlers;
use crate::semantic::{GuardrailCheckEvent, GuardrailFault, GuardrailsState};
use crate::GuardrailsEngine;
use std::sync::Arc;

pub struct GuardrailsPlugin;

impl GuardrailsPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GuardrailsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for GuardrailsPlugin {
    fn id(&self) -> &str {
        "vil-guardrails"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "PII detection, toxicity scoring, and custom rules"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "guardrails".into(),
            endpoints: vec![
                EndpointSpec::post("/api/guardrails/validate"),
                EndpointSpec::get("/api/guardrails/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let engine = Arc::new(GuardrailsEngine::new());

        let svc = ServiceProcess::new("guardrails")
            .endpoint(Method::POST, "/validate", post(handlers::validate_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(engine)
            .emits::<GuardrailCheckEvent>()
            .faults::<GuardrailFault>()
            .manages::<GuardrailsState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
