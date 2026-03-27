use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::entity::{Entity, EntityType};
use crate::relation::{Relation, RelationType};

/// Persistent knowledge graph for agent long-term memory.
///
/// Thread-safe: all operations use interior mutability via `RwLock` and `DashMap`.
pub struct MemoryGraph {
    entities: RwLock<Vec<Entity>>,
    relations: RwLock<Vec<Relation>>,
    name_index: DashMap<String, u64>,
    next_id: AtomicU64,
}

impl MemoryGraph {
    /// Create an empty memory graph.
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
            relations: RwLock::new(Vec::new()),
            name_index: DashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    fn now_ts() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Add an entity to the graph. Returns its unique ID.
    pub fn add_entity(
        &self,
        name: &str,
        entity_type: EntityType,
        attributes: serde_json::Value,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Self::now_ts();
        let entity = Entity {
            id,
            name: name.to_string(),
            entity_type,
            attributes,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            importance: 0.5,
        };
        self.name_index.insert(name.to_lowercase(), id);
        self.entities.write().push(entity);
        id
    }

    /// Add a directed relation between two entities.
    pub fn add_relation(&self, from: u64, to: u64, rel_type: RelationType, weight: f32) {
        let relation = Relation {
            from,
            to,
            relation_type: rel_type,
            weight: weight.clamp(0.0, 1.0),
            created_at: Self::now_ts(),
            metadata: serde_json::Value::Null,
        };
        self.relations.write().push(relation);
    }

    /// Find an entity by exact name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Option<Entity> {
        let key = name.to_lowercase();
        let id = *self.name_index.get(&key)?;
        self.get_entity(id)
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, id: u64) -> Option<Entity> {
        let entities = self.entities.read();
        entities.iter().find(|e| e.id == id).cloned()
    }

    /// Get all relations where the entity is source or target.
    pub fn relations_of(&self, entity_id: u64) -> Vec<Relation> {
        let relations = self.relations.read();
        relations
            .iter()
            .filter(|r| r.from == entity_id || r.to == entity_id)
            .cloned()
            .collect()
    }

    /// Get directly connected entities (neighbors).
    pub fn neighbors(&self, entity_id: u64) -> Vec<Entity> {
        let rels = self.relations_of(entity_id);
        let entities = self.entities.read();
        let mut neighbor_ids: Vec<u64> = rels
            .iter()
            .map(|r| if r.from == entity_id { r.to } else { r.from })
            .collect();
        neighbor_ids.sort_unstable();
        neighbor_ids.dedup();
        neighbor_ids
            .iter()
            .filter_map(|id| entities.iter().find(|e| e.id == *id).cloned())
            .collect()
    }

    /// Find all entities of a given type.
    pub fn find_by_type(&self, entity_type: &EntityType) -> Vec<Entity> {
        let entities = self.entities.read();
        entities
            .iter()
            .filter(|e| e.entity_type == *entity_type)
            .cloned()
            .collect()
    }

    /// Recall the most relevant entities for a free-text query.
    ///
    /// Scoring: `name_match * 0.4 + importance * 0.3 + recency * 0.3`
    pub fn recall(&self, query: &str, top_k: usize) -> Vec<Entity> {
        let entities = self.entities.read();
        if entities.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return Vec::new();
        }

        let now = Self::now_ts();
        // Find the maximum age to normalize recency.
        let max_age = entities
            .iter()
            .map(|e| now.saturating_sub(e.last_accessed))
            .max()
            .unwrap_or(1)
            .max(1) as f32;

        let mut scored: Vec<(f32, &Entity)> = entities
            .iter()
            .map(|e| {
                let name_lower = e.name.to_lowercase();
                let matched = query_words
                    .iter()
                    .filter(|w| name_lower.contains(**w))
                    .count();
                let name_match = matched as f32 / query_words.len() as f32;

                let age = now.saturating_sub(e.last_accessed) as f32;
                let recency = 1.0 - (age / max_age);

                let score = name_match * 0.4 + e.importance * 0.3 + recency * 0.3;
                (score, e)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, e)| e.clone()).collect()
    }

    /// Total number of entities.
    pub fn entity_count(&self) -> usize {
        self.entities.read().len()
    }

    /// Total number of relations.
    pub fn relation_count(&self) -> usize {
        self.relations.read().len()
    }

    /// Touch an entity: update `last_accessed` and increment `access_count`.
    pub fn touch(&self, entity_id: u64) {
        let mut entities = self.entities.write();
        if let Some(e) = entities.iter_mut().find(|e| e.id == entity_id) {
            e.last_accessed = Self::now_ts();
            e.access_count += 1;
        }
    }

    // --- Internal helpers used by decay / store modules ---

    /// Mutable access to entities (crate-internal).
    pub(crate) fn entities_mut(&self) -> &RwLock<Vec<Entity>> {
        &self.entities
    }

    /// Read access to entities (crate-internal).
    pub(crate) fn entities_ref(&self) -> &RwLock<Vec<Entity>> {
        &self.entities
    }

    /// Mutable access to relations (crate-internal).
    pub(crate) fn relations_mut(&self) -> &RwLock<Vec<Relation>> {
        &self.relations
    }

    /// Read access to relations (crate-internal).
    pub(crate) fn relations_ref(&self) -> &RwLock<Vec<Relation>> {
        &self.relations
    }

    /// Reference to the name index (crate-internal).
    pub(crate) fn name_index_ref(&self) -> &DashMap<String, u64> {
        &self.name_index
    }

    /// Reference to next_id counter (crate-internal).
    pub(crate) fn next_id_ref(&self) -> &AtomicU64 {
        &self.next_id
    }
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}
