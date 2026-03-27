// =============================================================================
// vil_queue::descriptor_queue — Lock-Free MPMC Descriptor Queue
// =============================================================================
// Default QueueBackend implementation using lock-free SegQueue.
// Safe, debuggable, and suitable for general use.
//
// This queue carries ONLY Descriptors (sample_id, origin_port, lineage_id,
// region_id) — not large payloads. Payloads live in the shared exchange heap.
//
// TASK LIST:
// [x] DescriptorQueue struct
// [x] QueueBackend impl
// [x] Clone support (Arc-wrapped)
// [x] Unit tests
// =============================================================================

use crossbeam_queue::SegQueue;
use std::sync::Arc;

use vil_types::Descriptor;

use crate::traits::QueueBackend;

/// High-performance concurrent descriptor queue.
///
/// Lock-free MPMC via SegQueue.
/// Eliminates Mutex contention when many sessions publish to the same out_port.
#[derive(Clone, Default)]
pub struct DescriptorQueue {
    inner: Arc<SegQueue<Descriptor>>,
}

impl DescriptorQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SegQueue::new()),
        }
    }

    /// Create a queue with capacity (SegQueue is unbounded, but API is kept for compatibility).
    #[doc(alias = "vil_keep")]
    pub fn with_capacity(_capacity: usize) -> Self {
        Self::new()
    }
}

impl QueueBackend for DescriptorQueue {
    fn push(&self, descriptor: Descriptor) {
        self.inner.push(descriptor);
    }

    fn try_pop(&self) -> Option<Descriptor> {
        self.inner.pop()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_types::{HostId, PortId, SampleId};

    fn make_descriptor(id: u64) -> Descriptor {
        Descriptor {
            sample_id: SampleId(id),
            origin_host: HostId(0),
            origin_port: PortId(1),
            lineage_id: id * 10,
            publish_ts: 0,
        }
    }

    #[test]
    fn test_push_and_pop_fifo_order() {
        let q = DescriptorQueue::new();
        q.push(make_descriptor(1));
        q.push(make_descriptor(2));
        q.push(make_descriptor(3));

        assert_eq!(q.len(), 3);

        let d1 = q.try_pop().unwrap();
        assert_eq!(d1.sample_id, SampleId(1));

        let d2 = q.try_pop().unwrap();
        assert_eq!(d2.sample_id, SampleId(2));

        let d3 = q.try_pop().unwrap();
        assert_eq!(d3.sample_id, SampleId(3));

        assert!(q.try_pop().is_none());
    }

    #[test]
    fn test_empty_queue() {
        let q = DescriptorQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert!(q.try_pop().is_none());
    }

    #[test]
    fn test_with_capacity() {
        let q = DescriptorQueue::with_capacity(100);
        assert!(q.is_empty());
        q.push(make_descriptor(1));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_clone_shares_state() {
        let q1 = DescriptorQueue::new();
        let q2 = q1.clone();

        q1.push(make_descriptor(42));
        let d = q2.try_pop().unwrap();
        assert_eq!(d.sample_id, SampleId(42));
    }

    #[test]
    fn test_queue_backend_trait_object() {
        let q: Box<dyn QueueBackend> = Box::new(DescriptorQueue::new());
        q.push(make_descriptor(99));
        assert_eq!(q.len(), 1);
        let d = q.try_pop().unwrap();
        assert_eq!(d.sample_id, SampleId(99));
    }
}
