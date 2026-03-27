use vil_server::prelude::*;

use std::sync::Arc;
use crate::{ChunkStrategy, SentenceChunker};
use crate::handlers;
use crate::vil_semantic::{ChunkEvent, ChunkFault, ChunkerState};

pub struct ChunkerPlugin {
    max_tokens: usize,
}

impl ChunkerPlugin {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }
}

impl Default for ChunkerPlugin {
    fn default() -> Self {
        Self { max_tokens: 512 }
    }
}

impl VilPlugin for ChunkerPlugin {
    fn id(&self) -> &str { "vil-chunker" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "Strategy-based text chunking for RAG pipelines" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "chunker".into(),
            endpoints: vec![
                EndpointSpec::post("/api/chunker/chunk"),
                EndpointSpec::get("/api/chunker/stats"),
            ],
        }]
    }

    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }

    fn register(&self, ctx: &mut PluginContext) {
        let chunker: Arc<dyn ChunkStrategy> = Arc::new(SentenceChunker::new(self.max_tokens));
        ctx.provide::<Arc<dyn ChunkStrategy>>("chunker-strategy", chunker.clone());

        let svc = ServiceProcess::new("chunker")
            .endpoint(Method::POST, "/chunk", post(handlers::chunk_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(chunker)
            .emits::<ChunkEvent>()
            .faults::<ChunkFault>()
            .manages::<ChunkerState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
