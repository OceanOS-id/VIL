use vil_server::prelude::*;

use crate::dataset::EvalDataset;
use crate::handlers;
use crate::semantic::{EvalFault, EvalRunEvent, EvalState};
use std::sync::Arc;

pub struct EvalPlugin;

impl EvalPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EvalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for EvalPlugin {
    fn id(&self) -> &str {
        "vil-eval"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "LLM evaluation framework with metrics and reporting"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "eval".into(),
            endpoints: vec![
                EndpointSpec::post("/api/eval/run"),
                EndpointSpec::get("/api/eval/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let dataset = Arc::new(EvalDataset::new());

        let svc = ServiceProcess::new("eval")
            .state(dataset)
            .endpoint(Method::POST, "/run", post(handlers::run_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<EvalRunEvent>()
            .faults::<EvalFault>()
            .manages::<EvalState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
