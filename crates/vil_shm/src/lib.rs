// =============================================================================
// vil_shm — Shared Exchange Heap
// =============================================================================
// Implements the shared exchange heap for VIL.
// All objects exchanged between processes are allocated here,
// NOT on process-local heaps.
//
// Architecture:
//   ┌─────────────────────────────────────────────────────────┐
//   │ Layer 1: ExchangeHeap (multi-region typed allocation)   │
//   │ Layer 2: BumpAllocator (O(1) atomic alloc per region)   │
//   │ Layer 3: Offset + RelativePtr<T> (address-independent)  │
//   │ Layer 4: SharedStore (backward-compat Arc-based store)  │
//   │ Layer 5: RegionManager (region metadata tracking)       │
//   └─────────────────────────────────────────────────────────┘
//
// Modules:
//   offset.rs    — Offset, RelativePtr<T> (address-independent referencing)
//   allocator.rs — BumpAllocator (atomic O(1) bump allocation)
//   heap.rs      — ExchangeHeap (multi-region typed API)
//   store.rs     — SharedStore (backward-compat HashMap+Arc store)
//   region.rs    — RegionManager (region metadata)
//
// TASK LIST:
// [x] Offset, RelativePtr<T> — relative addressing
// [x] BumpAllocator — atomic bump allocation, alignment, batch reset
// [x] ExchangeHeap — multi-region typed alloc/read/write
// [x] SharedStore — backward-compatible Arc-based store
// [x] RegionManager — region metadata tracking
// [ ] TODO(future): mmap-backed RegionSlot (MAP_SHARED|MAP_ANONYMOUS)
// [ ] TODO(future): SlabAllocator for fixed-size hot path
// [ ] TODO(future): sub-region compaction
// =============================================================================

pub mod allocator;
pub mod heap;
pub mod offset;
pub mod paged_allocator;
pub mod region;
pub mod store;

pub use allocator::BumpAllocator;
pub use heap::{ExchangeHeap, RegionStats};
pub use offset::{Offset, RelativePtr};
pub use paged_allocator::PagedAllocator;
pub use region::RegionManager;
pub use store::SharedStore;

/// Minimal metadata for sample defragmentation.
pub struct DefragSample {
    pub id: vil_types::SampleId,
    pub offset: u64,
    pub size: usize,
    pub align: usize,
}

/// Trait abstracting registry access during compaction.
/// Breaks the circular dependency between vil_shm and vil_registry.
pub trait DefragRegistry {
    fn get_active_samples(&self, region_id: vil_types::RegionId) -> Vec<DefragSample>;
    fn update_offset(&self, sample_id: vil_types::SampleId, new_offset: u64);
}
