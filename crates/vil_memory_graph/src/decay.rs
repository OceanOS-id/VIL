use std::time::{SystemTime, UNIX_EPOCH};

use crate::graph::MemoryGraph;

/// Apply temporal decay to every entity in the graph.
///
/// For each entity the new importance is:
///
/// ```text
/// decay_factor = e^(-decay_rate * days_since_last_access)
/// new_importance = max(importance * decay_factor, min_importance)
/// ```
pub fn apply_decay(graph: &MemoryGraph, decay_rate: f32, min_importance: f32) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut entities = graph.entities_mut().write();
    for entity in entities.iter_mut() {
        let secs_since = now.saturating_sub(entity.last_accessed) as f32;
        let days_since = secs_since / 86_400.0;
        let decay_factor = (-decay_rate * days_since).exp();
        entity.importance = (entity.importance * decay_factor).max(min_importance);
    }
}

/// Garbage-collect entities whose importance has fallen below `threshold`.
///
/// Orphaned relations (referencing removed entities) are also removed.
/// Returns the number of removed entities.
pub fn gc(graph: &MemoryGraph, threshold: f32) -> usize {
    let removed_ids: Vec<u64>;

    {
        let mut entities = graph.entities_mut().write();
        let before = entities.len();
        removed_ids = entities
            .iter()
            .filter(|e| e.importance < threshold)
            .map(|e| e.id)
            .collect();
        entities.retain(|e| e.importance >= threshold);
        let _ = before - entities.len(); // count
    }

    // Clean up name index.
    let name_index = graph.name_index_ref();
    name_index.retain(|_, id| !removed_ids.contains(id));

    // Clean up orphaned relations.
    {
        let mut relations = graph.relations_mut().write();
        relations.retain(|r| !removed_ids.contains(&r.from) && !removed_ids.contains(&r.to));
    }

    removed_ids.len()
}
