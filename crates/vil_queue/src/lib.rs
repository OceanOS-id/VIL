// =============================================================================
// vil_queue — Descriptor Queue
// =============================================================================
// Queues in VIL transport DESCRIPTORS, not large payloads.
// Payloads live in the shared exchange heap; queues carry only:
// sample_id, origin_port, lineage_id, region_id, and flags.
//
// Implementations:
//   - QueueBackend trait — abstraction allowing swappable implementations
//   - SpscQueue — lock-free SPSC ring buffer (GOLDEN PATH)
//   - DescriptorQueue — lock-free MPMC fallback (debug/general use)
//
// Modules:
//   traits.rs          — QueueBackend trait
//   spsc.rs            — Lock-free SPSC ring buffer (production)
//   descriptor_queue.rs — Lock-free MPMC via SegQueue (fallback/debug)
//
// TASK LIST:
// [x] QueueBackend trait
// [x] DescriptorQueue (SegQueue) — fallback
// [x] SpscRingBuffer<T> — lock-free generic ring buffer
// [x] SpscQueue — Arc-wrapped SPSC for Descriptor
// [x] Cache-line padding (128 byte)
// [x] Power-of-2 capacity + bitmask indexing
// [x] Acquire/Release atomic ordering
// [x] Cross-thread stress tests
// [ ] TODO(future): MPMC bounded queue
// [ ] TODO(future): hybrid wait strategy (spin -> yield -> park)
// =============================================================================

pub mod traits;
pub mod spsc;
pub mod descriptor_queue;

pub use traits::QueueBackend;
pub use spsc::{SpscQueue, SpscRingBuffer};
pub use descriptor_queue::DescriptorQueue;
