//! VilPlugin implementation for RAG integration.
//!
//! Depends on vil-llm plugin for LlmProvider and EmbeddingProvider.
//! Registers ServiceProcess with /ingest, /query, /stats endpoints.

use vil_server::prelude::*;

use std::sync::Arc;


use vil_llm::{EmbeddingProvider, LlmProvider};

use crate::chunk::{ChunkerStrategy, FixedChunker, MarkdownChunker, SemanticChunker};
use crate::config::{ChunkerType, StoreType};
use crate::extractors::Rag;
use crate::handlers;
use crate::pipeline::RagPipeline;
use crate::retriever::Retriever;
use crate::semantic::{RagFault, RagIndexState, RagIngestEvent, RagQueryEvent};
use crate::store::{InMemoryStore, VectorStore};

/// VIL RAG Plugin — retrieval-augmented generation pipeline.
///
/// # Example
/// ```ignore
/// VilApp::new("rag-service")
///     .plugin(LlmPlugin::new().openai(config).embedder_openai(key, model))
///     .plugin(RagPlugin::new().chunker(ChunkerType::Semantic { chunk_size: 512, overlap: 50 }))
///     .run().await;
/// ```
pub struct RagPlugin {
    chunker_type: ChunkerType,
    store_type: StoreType,
}

impl RagPlugin {
    pub fn new() -> Self {
        Self {
            chunker_type: ChunkerType::default(),
            store_type: StoreType::default(),
        }
    }

    pub fn chunker(mut self, chunker_type: ChunkerType) -> Self {
        self.chunker_type = chunker_type;
        self
    }

    pub fn store(mut self, store_type: StoreType) -> Self {
        self.store_type = store_type;
        self
    }

    fn build_chunker(&self) -> Arc<dyn ChunkerStrategy> {
        match &self.chunker_type {
            ChunkerType::Fixed {
                chunk_size,
                overlap,
            } => Arc::new(FixedChunker::new(*chunk_size, *overlap)),
            ChunkerType::Semantic {
                chunk_size,
                overlap,
            } => Arc::new(SemanticChunker::new(*chunk_size, *overlap)),
            ChunkerType::Markdown { chunk_size } => Arc::new(MarkdownChunker::new(*chunk_size)),
        }
    }

    fn build_store(&self) -> Arc<dyn VectorStore> {
        match &self.store_type {
            StoreType::InMemory => Arc::new(InMemoryStore::new()),
        }
    }
}

impl Default for RagPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VilPlugin for RagPlugin {
    fn id(&self) -> &str {
        "vil-rag"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Retrieval-Augmented Generation pipeline (chunk, embed, store, retrieve, generate)"
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Resource {
                type_name: "RagPipeline",
                name: "rag-pipeline".into(),
            },
            PluginCapability::Resource {
                type_name: "Retriever",
                name: "rag-retriever".into(),
            },
            PluginCapability::Service {
                name: "rag".into(),
                endpoints: vec![
                    EndpointSpec::post("/api/rag/ingest").with_description("Ingest a document"),
                    EndpointSpec::post("/api/rag/query").with_description("RAG query"),
                    EndpointSpec::get("/api/rag/stats").with_description("Index stats"),
                ],
            },
        ]
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency::required("vil-llm", ">=0.1")]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let llm = ctx.require::<Arc<dyn LlmProvider>>("llm").clone();
        let embedder = ctx.require::<Arc<dyn EmbeddingProvider>>("embedder").clone();

        let store = self.build_store();
        let chunker = self.build_chunker();

        let pipeline = RagPipeline::builder()
            .chunker(chunker)
            .embedder(embedder)
            .store(store.clone())
            .llm(llm)
            .build();

        // Provide resources for other plugins (vil_agent)
        ctx.provide::<Arc<dyn Retriever>>("rag-retriever", pipeline.retriever());
        let pipeline = Arc::new(pipeline);
        ctx.provide::<Arc<RagPipeline>>("rag-pipeline", pipeline.clone());

        // Build ServiceProcess with VIL handler pattern
        let svc = ServiceProcess::new("rag")
            .endpoint(Method::POST, "/ingest", post(handlers::ingest_handler))
            .endpoint(Method::POST, "/query", post(handlers::query_handler))
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(Rag::from(pipeline))
            .emits::<RagQueryEvent>()
            .emits::<RagIngestEvent>()
            .faults::<RagFault>()
            .manages::<RagIndexState>();

        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth {
        PluginHealth::Healthy
    }
}
