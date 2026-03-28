use vil_server::prelude::*;

use crate::dataset::PreferenceDataset;
use crate::handlers;
use crate::vil_semantic::{RlhfEvent, RlhfFault, RlhfState};
use std::sync::{Arc, RwLock};

pub struct RlhfPlugin;

impl RlhfPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RlhfPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for RlhfPlugin {
    fn id(&self) -> &str {
        "vil-rlhf"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Human feedback collection pipeline for RLHF/DPO training"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "rlhf".into(),
            endpoints: vec![EndpointSpec::get("/api/rlhf/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let dataset = Arc::new(RwLock::new(PreferenceDataset::new()));

        let svc = ServiceProcess::new("rlhf")
            .state(dataset)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<RlhfEvent>()
            .faults::<RlhfFault>()
            .manages::<RlhfState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
