// =============================================================================
// vil_shm::store — Thread-Safe Sample Store
// =============================================================================
// SharedStore holds samples published by producers.
// Consumers retrieve data via SampleId without copying (via Arc, Phase 1).
//
// Invariants:
// - One SampleId maps to one value
// - Thread-safe via DashMap (Phase 1.5)
// - Type-safe retrieval via Any + downcast
//
// TASK LIST:
// [x] insert_typed — store sample with generic type
// [x] get_typed — retrieve sample with type-safe downcast
// [x] remove — remove sample from store
// [x] contains — check sample existence
// [x] len — number of stored samples
// [x] Unit tests
// =============================================================================

use dashmap::DashMap;
use std::any::Any;
use std::sync::Arc;

use vil_types::SampleId;

/// Internal type for values stored in the shared store.
type SharedAny = Arc<dyn Any + Send + Sync>;

/// Thread-safe store for samples on the shared exchange heap.
///
/// DashMap-backed concurrent store.
/// Eliminates global Mutex contention on the sample hot path.
#[derive(Clone, Default)]
pub struct SharedStore {
    inner: Arc<DashMap<SampleId, SharedAny>>,
}

impl SharedStore {
    /// Create a new empty store.
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Store a sample with generic type.
    /// If the SampleId already exists, the old value is overwritten.
    pub fn insert_typed<T>(&self, id: SampleId, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.inner.insert(id, Arc::new(value));
    }

    /// Retrieve a sample with type-safe downcast.
    /// Returns None if the ID is missing or the type does not match.
    pub fn get_typed<T>(&self, id: SampleId) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let value = self.inner.get(&id)?.clone();
        Arc::downcast::<T>(value).ok()
    }

    /// Remove a sample from the store. Returns true if the sample existed and was removed.
    pub fn remove(&self, id: SampleId) -> bool {
        self.inner.remove(&id).is_some()
    }

    /// Check whether a sample exists in the store.
    pub fn contains(&self, id: SampleId) -> bool {
        self.inner.contains_key(&id)
    }

    /// Number of stored samples.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let store = SharedStore::new();
        store.insert_typed(SampleId(1), 42u64);

        let val = store.get_typed::<u64>(SampleId(1));
        assert_eq!(*val.unwrap(), 42);
    }

    #[test]
    fn test_type_mismatch_returns_none() {
        let store = SharedStore::new();
        store.insert_typed(SampleId(1), 42u64);

        // Trying to get as wrong type
        let val = store.get_typed::<String>(SampleId(1));
        assert!(val.is_none());
    }

    #[test]
    fn test_missing_id_returns_none() {
        let store = SharedStore::new();
        let val = store.get_typed::<u64>(SampleId(999));
        assert!(val.is_none());
    }

    #[test]
    fn test_remove() {
        let store = SharedStore::new();
        store.insert_typed(SampleId(1), "hello".to_string());
        assert!(store.contains(SampleId(1)));

        assert!(store.remove(SampleId(1)));
        assert!(!store.contains(SampleId(1)));
        assert!(!store.remove(SampleId(1))); // already removed
    }

    #[test]
    fn test_len_and_empty() {
        let store = SharedStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.insert_typed(SampleId(1), 1u32);
        store.insert_typed(SampleId(2), 2u32);
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
    }
}
