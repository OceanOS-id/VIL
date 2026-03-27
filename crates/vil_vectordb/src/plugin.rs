use vil_server::prelude::*;

use std::sync::Arc;
use crate::collection::Collection;
use crate::config::HnswConfig;
use crate::handlers;
use crate::semantic::{SearchEvent, IndexEvent, VectorDbFault, VectorDbState};

pub struct VectorDbPlugin {
    name: String,
    dimension: usize,
    config: HnswConfig,
}

impl VectorDbPlugin {
    pub fn new(name: impl Into<String>, dimension: usize) -> Self {
        Self { name: name.into(), dimension, config: HnswConfig::default() }
    }
    pub fn config(mut self, config: HnswConfig) -> Self {
        self.config = config; self
    }
}

impl VilPlugin for VectorDbPlugin {
    fn id(&self) -> &str { "vil-vectordb" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Native HNSW vector database for RAG pipelines" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "vectordb".into(),
            endpoints: vec![
                EndpointSpec::post("/api/vectordb/search"),
                EndpointSpec::post("/api/vectordb/index"),
                EndpointSpec::get("/api/vectordb/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let col = Arc::new(Collection::new(&self.name, self.dimension, self.config.clone()));
        ctx.provide::<Arc<Collection>>("vectordb", col.clone());

        let svc = ServiceProcess::new("vectordb")
            .endpoint(Method::POST, "/search", post(handlers::search_handler))
            .endpoint(Method::POST, "/index", post(handlers::index_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(col)
            .emits::<SearchEvent>()
            .emits::<IndexEvent>()
            .faults::<VectorDbFault>()
            .manages::<VectorDbState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
