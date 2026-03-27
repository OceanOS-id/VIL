use std::io;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use crate::entity::Entity;
use crate::graph::MemoryGraph;
use crate::relation::Relation;

/// Serializable snapshot of a `MemoryGraph`.
#[derive(Serialize, Deserialize)]
struct GraphSnapshot {
    entities: Vec<Entity>,
    relations: Vec<Relation>,
    next_id: u64,
}

/// Save the graph to a JSON file at `path`.
pub fn save_to_file(graph: &MemoryGraph, path: &str) -> Result<(), io::Error> {
    let json = to_json(graph);
    std::fs::write(path, json)
}

/// Load a graph from a JSON file at `path`.
pub fn load_from_file(path: &str) -> Result<MemoryGraph, io::Error> {
    let json = std::fs::read_to_string(path)?;
    from_json(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Serialize the graph to a JSON string.
pub fn to_json(graph: &MemoryGraph) -> String {
    let snapshot = GraphSnapshot {
        entities: graph.entities_ref().read().clone(),
        relations: graph.relations_ref().read().clone(),
        next_id: graph.next_id_ref().load(Ordering::Relaxed),
    };
    serde_json::to_string_pretty(&snapshot).expect("graph serialization should not fail")
}

/// Deserialize a graph from a JSON string.
pub fn from_json(json: &str) -> Result<MemoryGraph, serde_json::Error> {
    let snapshot: GraphSnapshot = serde_json::from_str(json)?;
    let graph = MemoryGraph::new();

    // Restore next_id.
    graph
        .next_id_ref()
        .store(snapshot.next_id, Ordering::Relaxed);

    // Restore entities and rebuild name index.
    {
        let mut entities = graph.entities_mut().write();
        for entity in snapshot.entities {
            graph
                .name_index_ref()
                .insert(entity.name.to_lowercase(), entity.id);
            entities.push(entity);
        }
    }

    // Restore relations.
    {
        let mut relations = graph.relations_mut().write();
        *relations = snapshot.relations;
    }

    Ok(graph)
}
