use vil_server::prelude::*;

use crate::handlers;
use crate::keyword::KeywordReranker;
use crate::reranker::Reranker;
use crate::semantic::{RerankEvent, RerankFault, RerankerState};
use std::sync::Arc;

pub struct RerankerPlugin;

impl RerankerPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RerankerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for RerankerPlugin {
    fn id(&self) -> &str {
        "vil-reranker"
    }
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &str {
        "Keyword, cross-encoder, and RRF reranking for RAG retrieval"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "reranker".into(),
            endpoints: vec![EndpointSpec::post("/api/reranker/rerank")],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let reranker: Arc<dyn Reranker> = Arc::new(KeywordReranker::new(1.0));

        let svc = ServiceProcess::new("reranker")
            .state(reranker)
            .endpoint(Method::POST, "/rerank", post(handlers::rerank_handler))
            .emits::<RerankEvent>()
            .faults::<RerankFault>()
            .manages::<RerankerState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
