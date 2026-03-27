//! VilPlugin implementation for graph-enhanced RAG integration.

use vil_server::prelude::*;

use std::sync::Arc;

use vil_memory_graph::prelude::MemoryGraph;

use crate::handlers;
use crate::semantic::{GraphRagEvent, GraphRagFault, GraphRagState};

/// GraphRAG plugin — knowledge-graph-enhanced retrieval.
pub struct GraphRagPlugin {
    graph: Arc<MemoryGraph>,
}

impl GraphRagPlugin {
    pub fn new(graph: Arc<MemoryGraph>) -> Self {
        Self { graph }
    }
}

impl Default for GraphRagPlugin {
    fn default() -> Self {
        Self {
            graph: Arc::new(MemoryGraph::new()),
        }
    }
}

impl VilPlugin for GraphRagPlugin {
    fn id(&self) -> &str {
        "vil-graphrag"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Knowledge-graph-enhanced retrieval-augmented generation"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "graphrag".into(),
            endpoints: vec![
                EndpointSpec::post("/api/graphrag/query").with_description("Graph-enhanced RAG query"),
                EndpointSpec::get("/api/graphrag/stats").with_description("GraphRAG stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency::required("vil-llm", "0.1"),
        ]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let svc = ServiceProcess::new("graphrag")
            .state(Arc::clone(&self.graph))
            .endpoint(Method::POST, "/query", post(handlers::query_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .emits::<GraphRagEvent>()
            .faults::<GraphRagFault>()
            .manages::<GraphRagState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
