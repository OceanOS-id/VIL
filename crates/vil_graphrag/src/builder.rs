use vil_memory_graph::prelude::*;
use vil_memory_graph::entity::EntityType;
use vil_memory_graph::relation::RelationType;

use crate::extractor::{EntityExtractor, ExtractedEntityType};

/// Builds a MemoryGraph from documents by extracting entities and creating relations.
pub struct GraphRagBuilder {
    graph: MemoryGraph,
}

impl GraphRagBuilder {
    /// Create a new builder with a fresh graph.
    pub fn new() -> Self {
        Self {
            graph: MemoryGraph::new(),
        }
    }

    /// Create a builder wrapping an existing graph.
    pub fn with_graph(graph: MemoryGraph) -> Self {
        Self { graph }
    }

    /// Process a document: extract entities and add them to the graph.
    /// Entities found in the same document are connected via `MentionedIn` relations.
    pub async fn process_document(
        &self,
        doc_id: &str,
        text: &str,
        extractor: &dyn EntityExtractor,
    ) -> usize {
        let extracted = extractor.extract(text).await;
        if extracted.is_empty() {
            return 0;
        }

        // Create a document node
        let doc_node = self.graph.add_entity(
            doc_id,
            EntityType::Custom("Document".into()),
            serde_json::json!({ "text_len": text.len() }),
        );

        let mut entity_ids = Vec::new();

        for ent in &extracted {
            let graph_type = match &ent.entity_type {
                ExtractedEntityType::Person => EntityType::Person,
                ExtractedEntityType::Organization => EntityType::Custom("Organization".into()),
                ExtractedEntityType::Date => EntityType::Event,
                ExtractedEntityType::Location => EntityType::Location,
                _ => EntityType::Custom(format!("{:?}", ent.entity_type)),
            };

            // Check if entity already exists
            let eid = if let Some(existing) = self.graph.find_by_name(&ent.text) {
                existing.id
            } else {
                self.graph.add_entity(
                    &ent.text,
                    graph_type,
                    serde_json::json!({ "confidence": ent.confidence }),
                )
            };

            // Link entity to document
            self.graph.add_relation(eid, doc_node, RelationType::MentionedIn, ent.confidence);
            entity_ids.push(eid);
        }

        // Connect co-occurring entities
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                self.graph.add_relation(
                    entity_ids[i],
                    entity_ids[j],
                    RelationType::RelatedTo,
                    0.5,
                );
            }
        }

        extracted.len()
    }

    /// Get a reference to the underlying graph.
    pub fn graph(&self) -> &MemoryGraph {
        &self.graph
    }

    /// Consume the builder and return the graph.
    pub fn into_graph(self) -> MemoryGraph {
        self.graph
    }
}

impl Default for GraphRagBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::KeywordEntityExtractor;

    #[tokio::test]
    async fn test_build_graph_from_doc() {
        let builder = GraphRagBuilder::new();
        let extractor = KeywordEntityExtractor::new();
        let count = builder
            .process_document(
                "doc1",
                "John Smith met Alice Brown at the conference on 2025-01-15.",
                &extractor,
            )
            .await;
        assert!(count > 0, "should extract entities");

        // Document node should exist
        let doc = builder.graph().find_by_name("doc1");
        assert!(doc.is_some());
    }

    #[tokio::test]
    async fn test_build_graph_empty_doc() {
        let builder = GraphRagBuilder::new();
        let extractor = KeywordEntityExtractor::new();
        let count = builder
            .process_document("empty", "no entities here", &extractor)
            .await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_multiple_docs_share_entities() {
        let builder = GraphRagBuilder::new();
        let extractor = KeywordEntityExtractor::new();
        builder
            .process_document("doc1", "John Smith is an engineer", &extractor)
            .await;
        builder
            .process_document("doc2", "John Smith joined the team", &extractor)
            .await;

        // John Smith should exist in graph
        let john = builder.graph().find_by_name("John Smith");
        assert!(john.is_some());
    }
}
