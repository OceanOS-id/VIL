// =============================================================================
// vil_feature_store — D16: Real-Time Feature Store
// =============================================================================
//! A concurrent, TTL-aware feature store for real-time ML feature serving.
//!
//! Provides a `DashMap`-backed store with:
//! - Typed feature keys (`entity_id:feature_name`)
//! - TTL-based automatic expiration
//! - Point-in-time snapshots
//! - Batch retrieval
//! - Lock-free concurrent access

pub mod handlers;
pub mod key;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;
pub mod snapshot;
pub mod store;
pub mod ttl;

// Re-exports
pub use key::FeatureKey;
pub use plugin::FeatureStorePlugin;
pub use semantic::{FeatureEvent, FeatureFault, FeatureStoreState};
pub use snapshot::FeatureSnapshot;
pub use store::{FeatureStore, FeatureValue};

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn make_value(data: Vec<f32>) -> FeatureValue {
        FeatureValue {
            data,
            version: 0,
            created_at: 0, // will be set by store
            ttl_ms: None,
        }
    }

    fn make_value_with_ttl(data: Vec<f32>, ttl_ms: u64) -> FeatureValue {
        FeatureValue {
            data,
            version: 0,
            created_at: 0,
            ttl_ms: Some(ttl_ms),
        }
    }

    // ── Basic set/get ─────────────────────────────────────────────────

    #[test]
    fn test_set_and_get() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("user_1", "click_rate");
        store.set(&key, make_value(vec![0.5, 0.3, 0.2]));

        let val = store.get(&key).expect("should exist");
        assert_eq!(val.data, vec![0.5, 0.3, 0.2]);
    }

    #[test]
    fn test_missing_key_returns_none() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("nonexistent", "feature");
        assert!(store.get(&key).is_none());
    }

    // ── Batch get ─────────────────────────────────────────────────────

    #[test]
    fn test_batch_get() {
        let store = FeatureStore::new();
        let k1 = FeatureKey::new("u1", "f1");
        let k2 = FeatureKey::new("u2", "f2");
        let k3 = FeatureKey::new("u3", "f3"); // not set

        store.set(&k1, make_value(vec![1.0]));
        store.set(&k2, make_value(vec![2.0]));

        let results = store.get_batch(&[k1, k2, k3]);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());
        assert_eq!(results[0].as_ref().unwrap().data, vec![1.0]);
        assert_eq!(results[1].as_ref().unwrap().data, vec![2.0]);
    }

    // ── Delete ────────────────────────────────────────────────────────

    #[test]
    fn test_delete() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("user_1", "feat");
        store.set(&key, make_value(vec![1.0]));
        assert_eq!(store.count(), 1);

        let removed = store.delete(&key);
        assert!(removed);
        assert!(store.get(&key).is_none());
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("x", "y");
        assert!(!store.delete(&key));
    }

    // ── Count ─────────────────────────────────────────────────────────

    #[test]
    fn test_count() {
        let store = FeatureStore::new();
        assert_eq!(store.count(), 0);

        store.set(&FeatureKey::new("a", "f1"), make_value(vec![1.0]));
        store.set(&FeatureKey::new("b", "f2"), make_value(vec![2.0]));
        assert_eq!(store.count(), 2);
    }

    // ── TTL expiry ────────────────────────────────────────────────────

    #[test]
    fn test_ttl_expiry() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("user_1", "temp_feat");

        // Set with 50ms TTL
        store.set(&key, make_value_with_ttl(vec![1.0], 50));

        // Should be available immediately
        assert!(store.get(&key).is_some());

        // Wait for expiry
        thread::sleep(std::time::Duration::from_millis(80));

        // Should be expired now
        assert!(store.get(&key).is_none());
    }

    #[test]
    fn test_evict_expired() {
        let store = FeatureStore::new();
        let k1 = FeatureKey::new("u1", "short");
        let k2 = FeatureKey::new("u2", "long");

        store.set(&k1, make_value_with_ttl(vec![1.0], 50));
        store.set(&k2, make_value(vec![2.0])); // no TTL = never expires

        thread::sleep(std::time::Duration::from_millis(80));

        let evicted = store.evict_expired();
        assert_eq!(evicted, 1);
        assert_eq!(store.count(), 1);
        assert!(store.get(&k2).is_some());
    }

    // ── Overwrite updates version ─────────────────────────────────────

    #[test]
    fn test_overwrite_increments_version() {
        let store = FeatureStore::new();
        let key = FeatureKey::new("user_1", "feat");

        store.set(&key, make_value(vec![1.0]));
        let v1 = store.get(&key).unwrap().version;

        store.set(&key, make_value(vec![2.0]));
        let v2 = store.get(&key).unwrap().version;

        assert_eq!(v2, v1 + 1);
        assert_eq!(store.get(&key).unwrap().data, vec![2.0]);
    }

    // ── Snapshot ──────────────────────────────────────────────────────

    #[test]
    fn test_snapshot() {
        let store = FeatureStore::new();
        store.set(&FeatureKey::new("u1", "f1"), make_value(vec![1.0, 2.0]));
        store.set(&FeatureKey::new("u2", "f2"), make_value(vec![3.0]));

        let snap = store.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(!snap.is_empty());
        assert!(snap.taken_at > 0);
    }

    #[test]
    fn test_snapshot_excludes_expired() {
        let store = FeatureStore::new();
        store.set(
            &FeatureKey::new("u1", "short"),
            make_value_with_ttl(vec![1.0], 50),
        );
        store.set(&FeatureKey::new("u2", "long"), make_value(vec![2.0]));

        thread::sleep(std::time::Duration::from_millis(80));

        let snap = store.snapshot();
        assert_eq!(snap.len(), 1);
    }

    // ── Concurrent set/get ────────────────────────────────────────────

    #[test]
    fn test_concurrent_set_get() {
        use std::sync::Arc;

        let store = Arc::new(FeatureStore::new());
        let mut handles = vec![];

        // Spawn 10 writer threads
        for i in 0..10 {
            let s = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let key = FeatureKey::new(format!("entity_{}", i), "feat");
                s.set(&key, make_value(vec![i as f32]));
            }));
        }

        // Wait for all writers
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(store.count(), 10);

        // Read all back
        for i in 0..10 {
            let key = FeatureKey::new(format!("entity_{}", i), "feat");
            let val = store.get(&key).expect("should exist");
            assert_eq!(val.data, vec![i as f32]);
        }
    }

    // ── Key tests ─────────────────────────────────────────────────────

    #[test]
    fn test_feature_key_compound() {
        let key = FeatureKey::new("user_42", "embedding_v2");
        assert_eq!(key.to_compound(), "user_42:embedding_v2");
        assert_eq!(format!("{}", key), "user_42:embedding_v2");
    }

    #[test]
    fn test_feature_key_from_compound() {
        let key = FeatureKey::from_compound("user_42:embedding_v2").unwrap();
        assert_eq!(key.entity_id, "user_42");
        assert_eq!(key.feature_name, "embedding_v2");
    }

    #[test]
    fn test_feature_key_from_compound_invalid() {
        assert!(FeatureKey::from_compound("nodelimiter").is_none());
        assert!(FeatureKey::from_compound(":empty_entity").is_none());
    }

    // ── Default TTL ───────────────────────────────────────────────────

    #[test]
    fn test_store_default_ttl() {
        let store = FeatureStore::with_default_ttl(50);
        let key = FeatureKey::new("u1", "f1");
        store.set(&key, make_value(vec![1.0]));

        // Should be available immediately
        assert!(store.get(&key).is_some());

        thread::sleep(std::time::Duration::from_millis(80));

        // Should be expired via default TTL
        assert!(store.get(&key).is_none());
    }
}
