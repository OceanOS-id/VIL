use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// A single vector record with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorRecord {
    pub id: u64,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
    pub text: Option<String>,
}

/// Thread-safe document/vector storage backed by DashMap.
pub struct VectorStorage {
    records: DashMap<u64, VectorRecord>,
    next_id: AtomicU64,
}

impl VectorStorage {
    /// Create a new empty storage.
    pub fn new() -> Self {
        Self {
            records: DashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Insert a record. Returns the record's ID.
    pub fn insert(&self, record: VectorRecord) -> u64 {
        let id = record.id;
        self.records.insert(id, record);
        id
    }

    /// Get a clone of the record by ID.
    pub fn get(&self, id: u64) -> Option<VectorRecord> {
        self.records.get(&id).map(|r| r.clone())
    }

    /// Delete a record by ID. Returns true if it existed.
    pub fn delete(&self, id: u64) -> bool {
        self.records.remove(&id).is_some()
    }

    /// Number of records stored.
    pub fn count(&self) -> usize {
        self.records.len()
    }

    /// Allocate and return the next unique ID.
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for VectorStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: u64) -> VectorRecord {
        VectorRecord {
            id,
            vector: vec![1.0, 2.0, 3.0],
            metadata: serde_json::json!({"key": "value"}),
            text: Some("hello".to_string()),
        }
    }

    #[test]
    fn insert_and_get() {
        let storage = VectorStorage::new();
        let id = storage.insert(make_record(1));
        assert_eq!(id, 1);
        let rec = storage.get(1).unwrap();
        assert_eq!(rec.id, 1);
        assert_eq!(rec.text, Some("hello".to_string()));
    }

    #[test]
    fn get_nonexistent() {
        let storage = VectorStorage::new();
        assert!(storage.get(999).is_none());
    }

    #[test]
    fn delete_existing() {
        let storage = VectorStorage::new();
        storage.insert(make_record(1));
        assert!(storage.delete(1));
        assert!(storage.get(1).is_none());
    }

    #[test]
    fn delete_nonexistent() {
        let storage = VectorStorage::new();
        assert!(!storage.delete(42));
    }

    #[test]
    fn count() {
        let storage = VectorStorage::new();
        assert_eq!(storage.count(), 0);
        storage.insert(make_record(1));
        storage.insert(make_record(2));
        assert_eq!(storage.count(), 2);
        storage.delete(1);
        assert_eq!(storage.count(), 1);
    }

    #[test]
    fn next_id_increments() {
        let storage = VectorStorage::new();
        assert_eq!(storage.next_id(), 1);
        assert_eq!(storage.next_id(), 2);
        assert_eq!(storage.next_id(), 3);
    }
}
