use vil_server::prelude::*;

use crate::config::FederatedConfig;
use crate::federation::FederatedRetriever;
use crate::handlers;
use crate::vil_semantic::{FederatedEvent, FederatedFault, FederatedState};
use std::sync::Arc;

pub struct FederatedRagPlugin;

impl FederatedRagPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FederatedRagPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for FederatedRagPlugin {
    fn id(&self) -> &str {
        "vil-federated-rag"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Cross-silo federated retrieval with score-based merging"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "federated-rag".into(),
            endpoints: vec![EndpointSpec::get("/api/federated-rag/stats")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let retriever = Arc::new(FederatedRetriever::new(FederatedConfig::default()));

        let svc = ServiceProcess::new("federated-rag")
            .state(retriever)
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<FederatedEvent>()
            .faults::<FederatedFault>()
            .manages::<FederatedState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
