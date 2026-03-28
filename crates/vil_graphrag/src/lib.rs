//! # VIL Knowledge Graph RAG (I02)
//!
//! Graph-enhanced retrieval-augmented generation. Extract entities from text,
//! build a knowledge graph (backed by `vil_memory_graph`), and perform
//! graph-enhanced queries that combine entity traversal with keyword recall.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vil_graphrag::{GraphRagBuilder, KeywordEntityExtractor, GraphRagQuery};
//!
//! # async fn example() {
//! let builder = GraphRagBuilder::new();
//! let extractor = KeywordEntityExtractor::new();
//!
//! builder.process_document("doc1", "Alice Smith works at Acme Corp", &extractor).await;
//!
//! let graph = builder.into_graph();
//! let query = GraphRagQuery::new(&graph);
//! let result = query.query("Alice");
//! println!("Found {} entities", result.entities.len());
//! # }
//! ```

pub mod builder;
pub mod extractor;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod query;
pub mod semantic;

pub use builder::GraphRagBuilder;
pub use extractor::{
    EntityExtractor, ExtractedEntity, ExtractedEntityType, KeywordEntityExtractor,
};
pub use plugin::GraphRagPlugin;
pub use query::{EntityInfo, GraphRagQuery, GraphRagResult, RelationInfo};
pub use semantic::{GraphRagEvent, GraphRagFault, GraphRagFaultType, GraphRagState};
