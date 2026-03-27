// =============================================================================
// vil_queue::traits — Queue Backend Trait
// =============================================================================
// Queue abstraction allowing swappable implementations without changing
// consumer code.
//
// TASK LIST:
// [x] QueueBackend trait definition
// [ ] TODO(future): QueueBackendSpsc — SPSC lock-free impl
// [ ] TODO(future): QueueBackendMpmc — MPMC bounded impl
// =============================================================================

use vil_types::Descriptor;

/// Queue backend abstraction for descriptor transport.
///
/// Implementors must be thread-safe (Send + Sync) since queues are accessed
/// from different producer and consumer threads.
///
/// # Contract
/// - `push` must not panic (backpressure handled at a higher level)
/// - `try_pop` is non-blocking: returns None if queue is empty
/// - Ordering: FIFO (first-in, first-out)
pub trait QueueBackend: Send + Sync {
    /// Enqueue a descriptor at the back of the queue.
    fn push(&self, descriptor: Descriptor);

    /// Dequeue a descriptor from the front. Non-blocking.
    fn try_pop(&self) -> Option<Descriptor>;

    /// Current number of descriptors in the queue.
    fn len(&self) -> usize;

    /// Whether the queue is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
