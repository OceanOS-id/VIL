//! # VIL Memory Graph
//!
//! Persistent knowledge graph for agent long-term memory.
//! Agents remember across sessions, not just within a sliding window.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_memory_graph::prelude::*;
//!
//! let graph = MemoryGraph::new();
//!
//! let alice = graph.add_entity("Alice", EntityType::Person, serde_json::json!({"role": "engineer"}));
//! let rust  = graph.add_entity("Rust", EntityType::Concept, serde_json::json!({}));
//!
//! graph.add_relation(alice, rust, RelationType::RelatedTo, 0.9);
//!
//! let results = graph.recall("Alice", 5);
//! assert!(!results.is_empty());
//! ```

pub mod decay;
pub mod entity;
pub mod graph;
pub mod query;
pub mod relation;
pub mod store;

/// Convenience re-exports.
pub mod prelude {
    pub use crate::decay::{apply_decay, gc};
    pub use crate::entity::{Entity, EntityType};
    pub use crate::graph::MemoryGraph;
    pub use crate::query::{find_related, shortest_path};
    pub use crate::relation::{Relation, RelationType};
    pub use crate::store::{from_json, load_from_file, save_to_file, to_json};
}

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::MemoryGraphPlugin;
pub use semantic::{MemoryEvent, MemoryFault, MemoryState};

#[cfg(test)]
mod tests {
    use super::prelude::*;

    fn make_graph() -> MemoryGraph {
        let g = MemoryGraph::new();
        let _alice = g.add_entity(
            "Alice",
            EntityType::Person,
            serde_json::json!({"role": "engineer"}),
        );
        let _bob = g.add_entity(
            "Bob",
            EntityType::Person,
            serde_json::json!({"role": "designer"}),
        );
        let _rust = g.add_entity("Rust Language", EntityType::Concept, serde_json::json!({}));
        g
    }

    // --- Entity CRUD ---

    #[test]
    fn test_add_and_find_by_name() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").expect("Alice should exist");
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.entity_type, EntityType::Person);
    }

    #[test]
    fn test_find_by_name_case_insensitive() {
        let g = make_graph();
        assert!(g.find_by_name("alice").is_some());
        assert!(g.find_by_name("ALICE").is_some());
        assert!(g.find_by_name("aLiCe").is_some());
    }

    #[test]
    fn test_get_entity_by_id() {
        let g = MemoryGraph::new();
        let id = g.add_entity("Test", EntityType::Fact, serde_json::json!(null));
        let e = g.get_entity(id).expect("should find by ID");
        assert_eq!(e.name, "Test");
    }

    #[test]
    fn test_find_by_type() {
        let g = make_graph();
        let people = g.find_by_type(&EntityType::Person);
        assert_eq!(people.len(), 2);
        let concepts = g.find_by_type(&EntityType::Concept);
        assert_eq!(concepts.len(), 1);
    }

    // --- Relations ---

    #[test]
    fn test_add_relation_and_relations_of() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").unwrap();
        let rust = g.find_by_name("Rust Language").unwrap();
        g.add_relation(alice.id, rust.id, RelationType::RelatedTo, 0.9);

        let rels = g.relations_of(alice.id);
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to, rust.id);
    }

    #[test]
    fn test_neighbors() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").unwrap();
        let bob = g.find_by_name("Bob").unwrap();
        let rust = g.find_by_name("Rust Language").unwrap();

        g.add_relation(alice.id, bob.id, RelationType::RelatedTo, 0.5);
        g.add_relation(alice.id, rust.id, RelationType::RelatedTo, 0.8);

        let neighbors = g.neighbors(alice.id);
        assert_eq!(neighbors.len(), 2);
    }

    // --- Recall ---

    #[test]
    fn test_recall_returns_relevant() {
        let g = make_graph();
        let results = g.recall("Alice", 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "Alice");
    }

    #[test]
    fn test_recall_empty_graph() {
        let g = MemoryGraph::new();
        let results = g.recall("anything", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_recall_empty_query() {
        let g = make_graph();
        let results = g.recall("", 5);
        assert!(results.is_empty());
    }

    // --- Touch ---

    #[test]
    fn test_touch_updates_access() {
        let g = MemoryGraph::new();
        let id = g.add_entity("Touchable", EntityType::Fact, serde_json::json!(null));
        let before = g.get_entity(id).unwrap();
        assert_eq!(before.access_count, 0);

        g.touch(id);
        let after = g.get_entity(id).unwrap();
        assert_eq!(after.access_count, 1);
        assert!(after.last_accessed >= before.last_accessed);
    }

    // --- Decay ---

    #[test]
    fn test_decay_reduces_importance() {
        let g = MemoryGraph::new();
        let id = g.add_entity("Old Memory", EntityType::Fact, serde_json::json!(null));

        // Artificially set last_accessed to 30 days ago.
        {
            let mut entities = g.entities_mut().write();
            let e = entities.iter_mut().find(|e| e.id == id).unwrap();
            e.last_accessed = e.last_accessed.saturating_sub(30 * 86_400);
            e.importance = 1.0;
        }

        apply_decay(&g, 0.1, 0.05);

        let e = g.get_entity(id).unwrap();
        assert!(e.importance < 1.0, "importance should have decayed");
        assert!(e.importance >= 0.05, "importance should not go below min");
    }

    // --- GC ---

    #[test]
    fn test_gc_removes_low_importance() {
        let g = MemoryGraph::new();
        let id1 = g.add_entity("Important", EntityType::Fact, serde_json::json!(null));
        let id2 = g.add_entity("Unimportant", EntityType::Fact, serde_json::json!(null));

        // Set importances.
        {
            let mut entities = g.entities_mut().write();
            entities
                .iter_mut()
                .find(|e| e.id == id1)
                .unwrap()
                .importance = 0.8;
            entities
                .iter_mut()
                .find(|e| e.id == id2)
                .unwrap()
                .importance = 0.01;
        }

        g.add_relation(id1, id2, RelationType::RelatedTo, 0.5);

        let removed = gc(&g, 0.1);
        assert_eq!(removed, 1);
        assert_eq!(g.entity_count(), 1);
        assert!(g.find_by_name("Unimportant").is_none());
        // Orphaned relation should also be removed.
        assert_eq!(g.relation_count(), 0);
    }

    // --- Store ---

    #[test]
    fn test_json_roundtrip() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").unwrap();
        let rust = g.find_by_name("Rust Language").unwrap();
        g.add_relation(alice.id, rust.id, RelationType::IsA, 0.7);

        let json = to_json(&g);
        let g2 = from_json(&json).expect("deserialization should succeed");

        assert_eq!(g2.entity_count(), g.entity_count());
        assert_eq!(g2.relation_count(), g.relation_count());
        assert!(g2.find_by_name("Alice").is_some());
        assert!(g2.find_by_name("Rust Language").is_some());
    }

    // --- Counts ---

    #[test]
    fn test_empty_graph_counts() {
        let g = MemoryGraph::new();
        assert_eq!(g.entity_count(), 0);
        assert_eq!(g.relation_count(), 0);
    }

    // --- Query helpers ---

    #[test]
    fn test_find_related() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").unwrap();
        let bob = g.find_by_name("Bob").unwrap();
        let rust = g.find_by_name("Rust Language").unwrap();

        g.add_relation(alice.id, bob.id, RelationType::RelatedTo, 0.5);
        g.add_relation(bob.id, rust.id, RelationType::RelatedTo, 0.5);

        let related = find_related(&g, alice.id, 2);
        assert_eq!(related.len(), 2); // Bob and Rust Language
    }

    #[test]
    fn test_shortest_path() {
        let g = make_graph();
        let alice = g.find_by_name("Alice").unwrap();
        let bob = g.find_by_name("Bob").unwrap();
        let rust = g.find_by_name("Rust Language").unwrap();

        g.add_relation(alice.id, bob.id, RelationType::RelatedTo, 0.5);
        g.add_relation(bob.id, rust.id, RelationType::RelatedTo, 0.5);

        let path = shortest_path(&g, alice.id, rust.id).expect("path should exist");
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], alice.id);
        assert_eq!(path[2], rust.id);

        // No path case.
        let g2 = MemoryGraph::new();
        let a = g2.add_entity("A", EntityType::Fact, serde_json::json!(null));
        let b = g2.add_entity("B", EntityType::Fact, serde_json::json!(null));
        assert!(shortest_path(&g2, a, b).is_none());
    }
}
