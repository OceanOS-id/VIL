// =============================================================================
// vil_shm::region — Region Manager (Stub)
// =============================================================================
// RegionManager tracks allocation regions on the exchange heap.
// Each message class or use-case can have its own region
// for isolation and allocation efficiency.
//
// Phase 1: metadata tracking only (no real memory region).
// Target evolution:
//   - fixed-size slabs per region
//   - variable arena segments
//   - preallocated pools for hot path
//
// TASK LIST:
// [x] RegionInfo — region metadata
// [x] RegionManager — create/get/list regions
// [x] Unit tests
// [ ] TODO(future): real memory allocation per region
// [ ] TODO(future): region-per-message-class strategy
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vil_types::RegionId;

/// Memory region type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionType {
    /// Anonymous region (local heap, Vec-backed).
    Anonymous,
    /// Named shared memory region (backed by /dev/shm/<name>).
    Named(String),
}

/// Metadata of a region on the exchange heap.
#[derive(Clone, Debug)]
pub struct RegionInfo {
    /// Region ID.
    pub id: RegionId,
    /// Descriptive region name.
    pub name: String,
    /// Target region capacity (number of slots). 0 = unbounded.
    pub capacity: usize,
    /// Backing store type (Anonymous vs Named).
    pub region_type: RegionType,
}

/// Manager for exchange heap regions.
///
/// Phase 1: metadata tracking only.
/// Target evolution: real memory allocation and slab management.
#[derive(Clone, Default)]
pub struct RegionManager {
    inner: Arc<Mutex<RegionManagerState>>,
}

#[derive(Default)]
struct RegionManagerState {
    next_id: u64,
    regions: HashMap<RegionId, RegionInfo>,
}

impl RegionManager {
    /// Create a new RegionManager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new anonymous region.
    pub fn create_region(&self, name: &str, capacity: usize) -> RegionId {
        self.create_region_ext(name, capacity, RegionType::Anonymous)
    }

    /// Create a new named (shared memory) region.
    pub fn create_named_region(&self, name: &str, capacity: usize) -> RegionId {
        self.create_region_ext(name, capacity, RegionType::Named(name.to_string()))
    }

    fn create_region_ext(&self, name: &str, capacity: usize, rtype: RegionType) -> RegionId {
        let mut guard = self.inner.lock().expect("region manager lock poisoned");
        let id = RegionId(guard.next_id);
        guard.next_id += 1;
        guard.regions.insert(
            id,
            RegionInfo {
                id,
                name: name.to_string(),
                capacity,
                region_type: rtype,
            },
        );
        id
    }

    /// Get region info by ID.
    pub fn get_region(&self, id: RegionId) -> Option<RegionInfo> {
        let guard = self.inner.lock().expect("region manager lock poisoned");
        guard.regions.get(&id).cloned()
    }

    /// List all registered regions.
    pub fn list_regions(&self) -> Vec<RegionInfo> {
        let guard = self.inner.lock().expect("region manager lock poisoned");
        guard.regions.values().cloned().collect()
    }

    /// Number of registered regions.
    pub fn count(&self) -> usize {
        let guard = self.inner.lock().expect("region manager lock poisoned");
        guard.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_region() {
        let mgr = RegionManager::new();
        let id = mgr.create_region("camera_frames", 1024);

        let info = mgr.get_region(id).unwrap();
        assert_eq!(info.name, "camera_frames");
        assert_eq!(info.capacity, 1024);
    }

    #[test]
    fn test_auto_increment_ids() {
        let mgr = RegionManager::new();
        let id1 = mgr.create_region("region_a", 100);
        let id2 = mgr.create_region("region_b", 200);
        assert_ne!(id1, id2);
        assert!(id2.0 > id1.0);
    }

    #[test]
    fn test_list_regions() {
        let mgr = RegionManager::new();
        mgr.create_region("r1", 10);
        mgr.create_region("r2", 20);

        let list = mgr.list_regions();
        assert_eq!(list.len(), 2);
        assert_eq!(mgr.count(), 2);
    }

    #[test]
    fn test_missing_region_returns_none() {
        let mgr = RegionManager::new();
        assert!(mgr.get_region(RegionId(999)).is_none());
    }
}
