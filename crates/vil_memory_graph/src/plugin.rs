use vil_server::prelude::*;

use crate::graph::MemoryGraph;
use crate::handlers;
use crate::semantic::{MemoryEvent, MemoryFault, MemoryState};
use std::sync::Arc;

pub struct MemoryGraphPlugin;

impl MemoryGraphPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl VilPlugin for MemoryGraphPlugin {
    fn id(&self) -> &str {
        "vil-memory-graph"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Persistent knowledge graph for agent long-term memory"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "memory-graph".into(),
            endpoints: vec![
                EndpointSpec::post("/api/memory/query"),
                EndpointSpec::get("/api/memory/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let graph = Arc::new(MemoryGraph::new());
        ctx.provide::<Arc<MemoryGraph>>("memory-graph", graph.clone());

        let svc = ServiceProcess::new("memory-graph")
            .endpoint(Method::POST, "/query", post(handlers::query_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(graph)
            .emits::<MemoryEvent>()
            .faults::<MemoryFault>()
            .manages::<MemoryState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
