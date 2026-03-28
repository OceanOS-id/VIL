use vil_server::prelude::*;

use std::sync::Arc;

use crate::generator::SyntheticGenerator;
use crate::handlers;
use crate::quality::QualityChecker;
use crate::template::{conversation_template, instruction_template, qa_template};
use crate::vil_semantic::{SyntheticEvent, SyntheticFault, SyntheticState};

pub struct SyntheticPlugin;

impl SyntheticPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SyntheticPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for SyntheticPlugin {
    fn id(&self) -> &str {
        "vil-synthetic"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Template-based synthetic data generation with quality checking"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "synthetic".into(),
            endpoints: vec![EndpointSpec::get("/api/synthetic/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let gen = Arc::new(SyntheticGenerator::new(
            vec![
                qa_template(),
                instruction_template(),
                conversation_template(),
            ],
            QualityChecker::default(),
        ));

        let svc = ServiceProcess::new("synthetic")
            .state(gen)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<SyntheticEvent>()
            .faults::<SyntheticFault>()
            .manages::<SyntheticState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
