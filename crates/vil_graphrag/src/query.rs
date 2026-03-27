use serde::{Deserialize, Serialize};
use vil_memory_graph::entity::Entity;
use vil_memory_graph::prelude::*;

/// Result of a graph-enhanced RAG query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRagResult {
    /// Entities found relevant to the query.
    pub entities: Vec<EntityInfo>,
    /// Relations between found entities.
    pub relations: Vec<RelationInfo>,
    /// Assembled context string for LLM consumption.
    pub context: String,
    /// Optional answer (populated if LLM is used).
    pub answer: Option<String>,
}

/// Lightweight entity info for query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: u64,
    pub name: String,
    pub entity_type: String,
}

/// Lightweight relation info for query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationInfo {
    pub from_name: String,
    pub to_name: String,
    pub relation_type: String,
    pub weight: f32,
}

impl From<&Entity> for EntityInfo {
    fn from(e: &Entity) -> Self {
        Self {
            id: e.id,
            name: e.name.clone(),
            entity_type: format!("{:?}", e.entity_type),
        }
    }
}

/// Graph-enhanced query engine.
pub struct GraphRagQuery<'a> {
    graph: &'a MemoryGraph,
    max_hops: usize,
    max_results: usize,
}

impl<'a> GraphRagQuery<'a> {
    pub fn new(graph: &'a MemoryGraph) -> Self {
        Self {
            graph,
            max_hops: 2,
            max_results: 10,
        }
    }

    pub fn max_hops(mut self, hops: usize) -> Self {
        self.max_hops = hops;
        self
    }

    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    /// Query the graph for entities related to the query string.
    /// Uses recall + graph traversal to find relevant context.
    pub fn query(&self, query_text: &str) -> GraphRagResult {
        // Step 1: Find seed entities via keyword recall
        let seeds = self.graph.recall(query_text, self.max_results);

        if seeds.is_empty() {
            return GraphRagResult {
                entities: Vec::new(),
                relations: Vec::new(),
                context: String::new(),
                answer: None,
            };
        }

        // Step 2: Expand via graph traversal
        let mut all_entities: Vec<EntityInfo> = Vec::new();
        let mut all_relations: Vec<RelationInfo> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for seed in &seeds {
            if seen_ids.insert(seed.id) {
                all_entities.push(EntityInfo::from(seed));
            }

            // Get related entities
            let related = find_related(self.graph, seed.id, self.max_hops);
            for rel_entity in &related {
                if seen_ids.insert(rel_entity.id) {
                    all_entities.push(EntityInfo::from(rel_entity));
                }
            }

            // Get relations
            let rels = self.graph.relations_of(seed.id);
            for rel in &rels {
                let from_name = self
                    .graph
                    .get_entity(rel.from)
                    .map(|e| e.name.clone())
                    .unwrap_or_default();
                let to_name = self
                    .graph
                    .get_entity(rel.to)
                    .map(|e| e.name.clone())
                    .unwrap_or_default();
                all_relations.push(RelationInfo {
                    from_name,
                    to_name,
                    relation_type: format!("{:?}", rel.relation_type),
                    weight: rel.weight,
                });
            }
        }

        // Step 3: Build context string
        let context = build_context(&all_entities, &all_relations);

        // Truncate to max_results
        all_entities.truncate(self.max_results);

        GraphRagResult {
            entities: all_entities,
            relations: all_relations,
            context,
            answer: None,
        }
    }
}

/// Build a text context from entities and relations for LLM consumption.
fn build_context(entities: &[EntityInfo], relations: &[RelationInfo]) -> String {
    let mut ctx = String::new();

    if !entities.is_empty() {
        ctx.push_str("Entities:\n");
        for e in entities {
            ctx.push_str(&format!("- {} ({})\n", e.name, e.entity_type));
        }
    }

    if !relations.is_empty() {
        ctx.push_str("\nRelations:\n");
        for r in relations {
            ctx.push_str(&format!(
                "- {} --[{}]--> {} (weight: {:.2})\n",
                r.from_name, r.relation_type, r.to_name, r.weight
            ));
        }
    }

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_memory_graph::entity::EntityType;
    use vil_memory_graph::relation::RelationType;

    fn make_test_graph() -> MemoryGraph {
        let g = MemoryGraph::new();
        let alice = g.add_entity("Alice", EntityType::Person, serde_json::json!({}));
        let bob = g.add_entity("Bob", EntityType::Person, serde_json::json!({}));
        let rust = g.add_entity("Rust", EntityType::Concept, serde_json::json!({}));
        g.add_relation(alice, rust, RelationType::RelatedTo, 0.9);
        g.add_relation(bob, rust, RelationType::RelatedTo, 0.8);
        g
    }

    #[test]
    fn test_query_returns_entities() {
        let graph = make_test_graph();
        let query = GraphRagQuery::new(&graph);
        let result = query.query("Alice");
        assert!(!result.entities.is_empty());
        assert!(result.entities.iter().any(|e| e.name == "Alice"));
    }

    #[test]
    fn test_query_empty_graph() {
        let graph = MemoryGraph::new();
        let query = GraphRagQuery::new(&graph);
        let result = query.query("anything");
        assert!(result.entities.is_empty());
        assert!(result.context.is_empty());
    }

    #[test]
    fn test_query_builds_context() {
        let graph = make_test_graph();
        let query = GraphRagQuery::new(&graph);
        let result = query.query("Alice");
        assert!(result.context.contains("Alice"));
    }

    #[test]
    fn test_build_context_format() {
        let entities = vec![EntityInfo {
            id: 1,
            name: "Alice".into(),
            entity_type: "Person".into(),
        }];
        let relations = vec![RelationInfo {
            from_name: "Alice".into(),
            to_name: "Rust".into(),
            relation_type: "RelatedTo".into(),
            weight: 0.9,
        }];
        let ctx = build_context(&entities, &relations);
        assert!(ctx.contains("Alice"));
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("RelatedTo"));
    }
}
