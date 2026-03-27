use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering as CmpOrdering;

use parking_lot::RwLock;

use crate::config::HnswConfig;
use crate::distance::distance;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the vector DB operations.
#[derive(Debug, thiserror::Error)]
pub enum VectorDbError {
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    #[error("duplicate id: {0}")]
    DuplicateId(u64),
}

// ---------------------------------------------------------------------------
// Search hit
// ---------------------------------------------------------------------------

/// A single search result from the HNSW index.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: u64,
    pub distance: f32,
    /// Similarity score: `1.0 - distance` (for cosine, 1.0 = identical).
    pub score: f32,
}

// ---------------------------------------------------------------------------
// Internal helpers for priority queue
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Candidate {
    idx: usize,
    dist: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.dist == other.dist
    }
}
impl Eq for Candidate {}

/// Min-heap candidate (closest first when popped).
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        other.dist.partial_cmp(&self.dist).unwrap_or(CmpOrdering::Equal)
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

/// Max-heap candidate (farthest first when popped).
#[derive(Clone)]
struct FarCandidate {
    idx: usize,
    dist: f32,
}

impl PartialEq for FarCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.dist == other.dist
    }
}
impl Eq for FarCandidate {}

impl Ord for FarCandidate {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.dist.partial_cmp(&other.dist).unwrap_or(CmpOrdering::Equal)
    }
}
impl PartialOrd for FarCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// HNSW node
// ---------------------------------------------------------------------------

struct HnswNode {
    id: u64,
    vector: Vec<f32>,
    /// `connections[level]` = list of neighbor indices into the `nodes` vec.
    connections: Vec<Vec<usize>>,
    #[allow(dead_code)]
    level: usize,
}

// ---------------------------------------------------------------------------
// HNSW index
// ---------------------------------------------------------------------------

/// Hierarchical Navigable Small World index for approximate nearest neighbor search.
pub struct HnswIndex {
    config: HnswConfig,
    nodes: RwLock<Vec<HnswNode>>,
    id_to_idx: RwLock<HashMap<u64, usize>>,
    entry_point: RwLock<Option<usize>>,
    max_level: RwLock<usize>,
    dimension: usize,
    ml: f64, // normalization factor: 1 / ln(M)
}

impl HnswIndex {
    /// Create a new HNSW index for vectors of the given dimension.
    pub fn new(dimension: usize, config: HnswConfig) -> Self {
        let ml = 1.0 / (config.m as f64).ln();
        Self {
            config,
            nodes: RwLock::new(Vec::new()),
            id_to_idx: RwLock::new(HashMap::new()),
            entry_point: RwLock::new(None),
            max_level: RwLock::new(0),
            dimension,
            ml,
        }
    }

    /// Insert a vector with the given external ID.
    pub fn insert(&self, id: u64, vector: Vec<f32>) -> Result<(), VectorDbError> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::DimensionMismatch {
                expected: self.dimension,
                got: vector.len(),
            });
        }

        // Check for duplicates
        {
            let map = self.id_to_idx.read();
            if map.contains_key(&id) {
                return Err(VectorDbError::DuplicateId(id));
            }
        }

        let new_level = self.random_level();

        let node = HnswNode {
            id,
            vector: vector.clone(),
            connections: vec![Vec::new(); new_level + 1],
            level: new_level,
        };

        // Add node
        let new_idx;
        {
            let mut nodes = self.nodes.write();
            new_idx = nodes.len();
            nodes.push(node);
        }
        {
            let mut map = self.id_to_idx.write();
            map.insert(id, new_idx);
        }

        // If first node, set as entry point
        let current_entry;
        {
            let ep = self.entry_point.read();
            current_entry = *ep;
        }

        if current_entry.is_none() {
            *self.entry_point.write() = Some(new_idx);
            *self.max_level.write() = new_level;
            return Ok(());
        }

        let mut ep_idx = current_entry.unwrap();
        let current_max_level = *self.max_level.read();

        // Phase 1: Traverse from top level down to new_level+1, greedy closest
        {
            let nodes = self.nodes.read();
            for level in (new_level + 1..=current_max_level).rev() {
                ep_idx = self.search_layer_greedy(&nodes, &vector, ep_idx, level);
            }
        }

        // Phase 2: For each level from min(new_level, current_max_level) down to 0,
        // find ef_construction nearest, connect
        let top = std::cmp::min(new_level, current_max_level);
        for level in (0..=top).rev() {
            let neighbors;
            {
                let nodes = self.nodes.read();
                let candidates = self.search_layer_ef(
                    &nodes,
                    &vector,
                    ep_idx,
                    self.config.ef_construction,
                    level,
                );
                // Select M closest
                neighbors = self.select_neighbors(&candidates, self.config.m);
                // Update entry point for next level
                if let Some(c) = candidates.first() {
                    ep_idx = c.idx;
                }
            }

            // Connect new node to neighbors and back-connect
            {
                let mut nodes = self.nodes.write();
                for &neighbor_idx in &neighbors {
                    // Forward connection: new_idx -> neighbor
                    if level < nodes[new_idx].connections.len() {
                        nodes[new_idx].connections[level].push(neighbor_idx);
                    }
                    // Backward connection: neighbor -> new_idx
                    if level < nodes[neighbor_idx].connections.len() {
                        nodes[neighbor_idx].connections[level].push(new_idx);
                        // Prune if too many connections
                        let max_conn = if level == 0 { self.config.m * 2 } else { self.config.m };
                        if nodes[neighbor_idx].connections[level].len() > max_conn {
                            self.prune_connections(&mut nodes, neighbor_idx, level, max_conn);
                        }
                    }
                }
            }
        }

        // Update entry point if new node has a higher level
        if new_level > current_max_level {
            *self.entry_point.write() = Some(new_idx);
            *self.max_level.write() = new_level;
        }

        Ok(())
    }

    /// Search for the top-K nearest neighbors to `query`.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit> {
        if k == 0 {
            return Vec::new();
        }

        let ep = {
            let ep = self.entry_point.read();
            match *ep {
                Some(idx) => idx,
                None => return Vec::new(),
            }
        };

        let nodes = self.nodes.read();
        let current_max_level = *self.max_level.read();

        // Phase 1: greedy descent from top to level 1
        let mut ep_idx = ep;
        for level in (1..=current_max_level).rev() {
            ep_idx = self.search_layer_greedy(&nodes, query, ep_idx, level);
        }

        // Phase 2: search at level 0 with ef_search
        let ef = std::cmp::max(self.config.ef_search, k);
        let candidates = self.search_layer_ef(&nodes, query, ep_idx, ef, 0);

        // Return top-k
        candidates
            .into_iter()
            .take(k)
            .map(|c| {
                let node = &nodes[c.idx];
                SearchHit {
                    id: node.id,
                    distance: c.dist,
                    score: 1.0 - c.dist,
                }
            })
            .collect()
    }

    /// Delete a vector by external ID. Returns true if it was found and removed.
    ///
    /// Note: This performs a soft delete by clearing the node's connections and
    /// removing references from neighbors. The node slot remains allocated.
    pub fn delete(&self, id: u64) -> bool {
        let idx;
        {
            let mut map = self.id_to_idx.write();
            match map.remove(&id) {
                Some(i) => idx = i,
                None => return false,
            }
        }

        let mut nodes = self.nodes.write();

        // Remove connections from this node and any back-references
        let levels = nodes[idx].connections.len();
        for level in 0..levels {
            let neighbors: Vec<usize> = nodes[idx].connections[level].clone();
            nodes[idx].connections[level].clear();
            for &neighbor_idx in &neighbors {
                if neighbor_idx < nodes.len() {
                    if level < nodes[neighbor_idx].connections.len() {
                        nodes[neighbor_idx].connections[level].retain(|&x| x != idx);
                    }
                }
            }
        }

        // If this was the entry point, pick a new one
        {
            let ep = *self.entry_point.read();
            if ep == Some(idx) {
                drop(nodes);
                let map = self.id_to_idx.read();
                let new_ep = map.values().next().copied();
                *self.entry_point.write() = new_ep;
                if new_ep.is_none() {
                    *self.max_level.write() = 0;
                }
            }
        }

        true
    }

    /// Number of vectors currently in the index.
    pub fn len(&self) -> usize {
        self.id_to_idx.read().len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Dimension of vectors in this index.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn random_level(&self) -> usize {
        let uniform: f64 = rand::random::<f64>();
        let level = (-uniform.ln() * self.ml).floor() as usize;
        // Cap at a reasonable max to avoid degenerate graphs
        std::cmp::min(level, 16)
    }

    /// Greedy search in a single layer — returns the index of the closest node.
    fn search_layer_greedy(
        &self,
        nodes: &[HnswNode],
        query: &[f32],
        entry: usize,
        level: usize,
    ) -> usize {
        let mut current = entry;
        let mut current_dist = self.dist(query, &nodes[current].vector);

        loop {
            let mut changed = false;
            let conns = if level < nodes[current].connections.len() {
                &nodes[current].connections[level]
            } else {
                break;
            };
            for &neighbor_idx in conns {
                if neighbor_idx >= nodes.len() {
                    continue;
                }
                let d = self.dist(query, &nodes[neighbor_idx].vector);
                if d < current_dist {
                    current = neighbor_idx;
                    current_dist = d;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        current
    }

    /// Search a layer with beam width `ef`, returning up to `ef` candidates sorted by distance.
    fn search_layer_ef(
        &self,
        nodes: &[HnswNode],
        query: &[f32],
        entry: usize,
        ef: usize,
        level: usize,
    ) -> Vec<Candidate> {
        let entry_dist = self.dist(query, &nodes[entry].vector);

        let mut visited = HashSet::new();
        visited.insert(entry);

        // Min-heap of candidates to explore
        let mut candidates = BinaryHeap::new();
        candidates.push(Candidate {
            idx: entry,
            dist: entry_dist,
        });

        // Max-heap of current best results
        let mut results = BinaryHeap::<FarCandidate>::new();
        results.push(FarCandidate {
            idx: entry,
            dist: entry_dist,
        });

        while let Some(closest) = candidates.pop() {
            // If the closest candidate is farther than the worst result, stop
            let worst_dist = results.peek().map(|r| r.dist).unwrap_or(f32::MAX);
            if closest.dist > worst_dist && results.len() >= ef {
                break;
            }

            let conns = if level < nodes[closest.idx].connections.len() {
                &nodes[closest.idx].connections[level]
            } else {
                continue;
            };

            for &neighbor_idx in conns {
                if neighbor_idx >= nodes.len() || !visited.insert(neighbor_idx) {
                    continue;
                }
                // Skip deleted nodes (those not in id_to_idx)
                let id_map = self.id_to_idx.read();
                if !id_map.values().any(|&v| v == neighbor_idx) {
                    // This is O(n) — for production you'd maintain a deleted set.
                    // But we only check for nodes that have been fully deleted.
                    // Actually, let's just check if the node still has an id in the map.
                    if !id_map.contains_key(&nodes[neighbor_idx].id) {
                        continue;
                    }
                }
                drop(id_map);

                let d = self.dist(query, &nodes[neighbor_idx].vector);
                let worst_dist = results.peek().map(|r| r.dist).unwrap_or(f32::MAX);

                if results.len() < ef || d < worst_dist {
                    candidates.push(Candidate {
                        idx: neighbor_idx,
                        dist: d,
                    });
                    results.push(FarCandidate {
                        idx: neighbor_idx,
                        dist: d,
                    });
                    if results.len() > ef {
                        results.pop(); // remove farthest
                    }
                }
            }
        }

        // Collect and sort by distance ascending
        let mut result_vec: Vec<Candidate> = results
            .into_iter()
            .map(|fc| Candidate {
                idx: fc.idx,
                dist: fc.dist,
            })
            .collect();
        result_vec.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap_or(CmpOrdering::Equal));
        result_vec
    }

    /// Select the closest `m` neighbors from a sorted candidate list.
    fn select_neighbors(&self, candidates: &[Candidate], m: usize) -> Vec<usize> {
        candidates.iter().take(m).map(|c| c.idx).collect()
    }

    /// Prune connections of a node at a given level to at most `max_conn`.
    fn prune_connections(
        &self,
        nodes: &mut Vec<HnswNode>,
        node_idx: usize,
        level: usize,
        max_conn: usize,
    ) {
        let node_vector = nodes[node_idx].vector.clone();
        let mut neighbor_dists: Vec<(usize, f32)> = nodes[node_idx].connections[level]
            .iter()
            .map(|&n| (n, self.dist(&node_vector, &nodes[n].vector)))
            .collect();
        neighbor_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(CmpOrdering::Equal));
        neighbor_dists.truncate(max_conn);
        nodes[node_idx].connections[level] = neighbor_dists.into_iter().map(|(idx, _)| idx).collect();
    }

    fn dist(&self, a: &[f32], b: &[f32]) -> f32 {
        distance(a, b, self.config.metric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HnswConfig;

    fn default_index(dim: usize) -> HnswIndex {
        HnswIndex::new(dim, HnswConfig::default())
    }

    #[test]
    fn insert_single() {
        let idx = default_index(3);
        idx.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        assert_eq!(idx.len(), 1);
        assert!(!idx.is_empty());
    }

    #[test]
    fn insert_dimension_mismatch() {
        let idx = default_index(3);
        let err = idx.insert(1, vec![1.0, 0.0]).unwrap_err();
        assert!(matches!(err, VectorDbError::DimensionMismatch { .. }));
    }

    #[test]
    fn insert_duplicate_id() {
        let idx = default_index(3);
        idx.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        let err = idx.insert(1, vec![0.0, 1.0, 0.0]).unwrap_err();
        assert!(matches!(err, VectorDbError::DuplicateId(1)));
    }

    #[test]
    fn search_empty_index() {
        let idx = default_index(3);
        let results = idx.search(&[1.0, 0.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_correct_order() {
        let idx = default_index(3);
        // Insert three vectors: query will be [1,0,0]
        idx.insert(1, vec![1.0, 0.0, 0.0]).unwrap(); // identical to query
        idx.insert(2, vec![0.9, 0.1, 0.0]).unwrap(); // very close
        idx.insert(3, vec![0.0, 1.0, 0.0]).unwrap(); // orthogonal

        let results = idx.search(&[1.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        // First result should be vector 1 (identical)
        assert_eq!(results[0].id, 1);
        assert!(results[0].distance < results[1].distance);
        assert!(results[1].distance < results[2].distance);
    }

    #[test]
    fn search_top_k_limits() {
        let idx = default_index(2);
        for i in 0..10 {
            idx.insert(i, vec![i as f32, 0.0]).unwrap();
        }
        let results = idx.search(&[5.0, 0.0], 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_k_zero() {
        let idx = default_index(2);
        idx.insert(1, vec![1.0, 0.0]).unwrap();
        let results = idx.search(&[1.0, 0.0], 0);
        assert!(results.is_empty());
    }

    #[test]
    fn delete_existing() {
        let idx = default_index(3);
        idx.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        assert!(idx.delete(1));
        assert_eq!(idx.len(), 0);
        assert!(idx.is_empty());
    }

    #[test]
    fn delete_nonexistent() {
        let idx = default_index(3);
        assert!(!idx.delete(42));
    }

    #[test]
    fn insert_multiple() {
        let idx = default_index(4);
        for i in 0..50 {
            let v = vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32];
            idx.insert(i, v).unwrap();
        }
        assert_eq!(idx.len(), 50);
        assert_eq!(idx.dimension(), 4);
    }

    #[test]
    fn concurrent_inserts() {
        use std::sync::Arc;
        use std::thread;

        let idx = Arc::new(default_index(3));
        let mut handles = Vec::new();

        for t in 0..4 {
            let idx_clone = Arc::clone(&idx);
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let id = (t * 25 + i) as u64;
                    let v = vec![id as f32, 0.0, 0.0];
                    idx_clone.insert(id, v).unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(idx.len(), 100);
    }
}
