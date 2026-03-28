// =============================================================================
// vil_queue::spsc — Lock-Free SPSC Ring Buffer
// =============================================================================
// Single-Producer Single-Consumer lock-free ring buffer.
// This is the golden-path queue for VIL descriptor transport.
//
// Design:
//   - Power-of-2 capacity with bitmasking (no modulo)
//   - AtomicUsize head (consumer) and tail (producer)
//   - Cache-line padded (128 byte) to prevent false sharing
//   - UnsafeCell<[MaybeUninit<T>; N]> backing store
//   - Ordering: Acquire/Release semantics (not SeqCst)
//   - Bounded: producer fails push when full (natural backpressure)
//
// Safety:
//   - ONLY safe for exactly 1 producer and 1 consumer
//   - Must NOT be used for MPMC (use DescriptorQueue for that)
//   - Send + Sync because access is governed by atomics
//
// TASK LIST:
// [x] CachePadded wrapper
// [x] SpscQueue struct
// [x] push / try_push (producer side)
// [x] pop / try_pop (consumer side)
// [x] len / is_empty / is_full / capacity
// [x] QueueBackend impl
// [x] Comprehensive unit tests
// [x] Cross-thread stress test
// =============================================================================

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nix::sys::eventfd::{eventfd, EfdFlags};
use std::os::unix::io::{IntoRawFd, RawFd};
use vil_types::Descriptor;

use crate::traits::QueueBackend;

// =============================================================================
// Cache-line padding
// =============================================================================

/// Cache-line padded value. Prevents false sharing between cores.
///
/// 128-byte padding accommodates modern architectures with
/// prefetch lines of 2x64 bytes (Intel) or 128 bytes (some ARM).
#[repr(align(128))]
struct CachePadded<T> {
    value: T,
}

impl<T> CachePadded<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

// =============================================================================
// SPSC Ring Buffer
// =============================================================================

/// Lock-free Single-Producer Single-Consumer ring buffer.
///
/// **ONLY safe for exactly 1 producer thread and 1 consumer thread.**
///
/// Capacity is always power-of-2 for bitmask indexing.
/// Head (consumer) and tail (producer) are cache-line-padded to
/// prevent false sharing.
pub struct SpscRingBuffer<T> {
    /// Buffer backing store. UnsafeCell because accessed from 2 threads
    /// (but never concurrently on the same slot).
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    /// Capacity (always power-of-2).
    capacity: usize,
    /// Bitmask = capacity - 1 (for fast modulo).
    mask: usize,
    /// Write position (only mutated by producer).
    tail: CachePadded<AtomicUsize>,
    /// Read position (only mutated by consumer).
    head: CachePadded<AtomicUsize>,
}

// Safety: SpscRingBuffer is safe to share across threads because:
// - head is only mutated by the consumer thread
// - tail is only mutated by the producer thread
// - atomics govern inter-thread visibility
// - T: Send is required because data moves between threads
unsafe impl<T: Send> Send for SpscRingBuffer<T> {}
unsafe impl<T: Send> Sync for SpscRingBuffer<T> {}

impl<T> SpscRingBuffer<T> {
    /// Create a new ring buffer with minimum capacity `min_capacity`.
    /// Actual capacity is rounded up to the nearest power-of-2.
    ///
    /// # Panics
    /// Panics if `min_capacity` == 0.
    pub fn new(min_capacity: usize) -> Self {
        assert!(min_capacity > 0, "SPSC capacity must be > 0");

        let capacity = min_capacity.next_power_of_two();
        let mask = capacity - 1;

        // Allocate buffer as Vec<UnsafeCell<MaybeUninit<T>>>
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            mask,
            tail: CachePadded::new(AtomicUsize::new(0)),
            head: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Queue capacity (power-of-2).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Try to push an item. Returns `Err(item)` if the queue is full.
    ///
    /// **MUST only be called from the producer thread.**
    pub fn try_push(&self, item: T) -> Result<(), T> {
        let tail = self.tail.value.load(Ordering::Relaxed);
        let head = self.head.value.load(Ordering::Acquire);

        // Queue is full if tail - head == capacity
        if tail.wrapping_sub(head) >= self.capacity {
            return Err(item);
        }

        // Write item to slot
        let idx = tail & self.mask;
        // SAFETY: idx is within buffer bounds (masked). Exclusive access guaranteed by single-producer invariant (head ownership).
        unsafe {
            (*self.buffer[idx].get()).write(item);
        }

        // Publish new tail — consumer will see this item
        self.tail
            .value
            .store(tail.wrapping_add(1), Ordering::Release);

        Ok(())
    }

    /// Push item. Spin-waits if queue is full.
    ///
    /// **MUST only be called from the producer thread.**
    ///
    /// Uses hybrid wait: spin then yield.
    pub fn push(&self, item: T) {
        let mut item = item;
        loop {
            match self.try_push(item) {
                Ok(()) => return,
                Err(returned) => {
                    item = returned;
                    std::hint::spin_loop();
                }
            }
        }
    }

    /// Try to pop an item. Returns `None` if the queue is empty.
    ///
    /// **MUST only be called from the consumer thread.**
    pub fn try_pop(&self) -> Option<T> {
        let head = self.head.value.load(Ordering::Relaxed);
        let tail = self.tail.value.load(Ordering::Acquire);

        // Queue is empty if head == tail
        if head == tail {
            return None;
        }

        // Read item from slot
        let idx = head & self.mask;
        // SAFETY: idx slot was previously written by producer. Exclusive access guaranteed by single-consumer invariant (tail ownership).
        let item = unsafe { (*self.buffer[idx].get()).assume_init_read() };

        // Publish new head — producer will see this slot as available
        self.head
            .value
            .store(head.wrapping_add(1), Ordering::Release);

        Some(item)
    }

    /// Number of items in the queue (snapshot, may change).
    pub fn len(&self) -> usize {
        let tail = self.tail.value.load(Ordering::Acquire);
        let head = self.head.value.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Whether the queue is empty (snapshot).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the queue is full (snapshot).
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }
}

impl<T> Drop for SpscRingBuffer<T> {
    fn drop(&mut self) {
        // Drain remaining items that were not popped to prevent leaks
        let head = *self.head.value.get_mut();
        let tail = *self.tail.value.get_mut();
        for i in head..tail {
            let idx = i & self.mask;
            // SAFETY: slots between head and tail were previously initialized.
            unsafe {
                self.buffer[idx].get_mut().assume_init_drop();
            }
        }
    }
}

// =============================================================================
// SpscQueue — Arc-wrapped SPSC for shared usage pattern
// =============================================================================

/// Arc-wrapped SPSC ring buffer for Descriptor transport.
///
/// Implements `QueueBackend` so it can be swapped with `DescriptorQueue`.
/// Cloneable (state shared via Arc).
#[derive(Clone)]
pub struct SpscQueue {
    inner: Arc<SpscRingBuffer<Descriptor>>,
}

impl SpscQueue {
    /// Create a new SPSC queue with minimum capacity `min_capacity`.
    #[doc(alias = "vil_keep")]
    pub fn new(min_capacity: usize) -> Self {
        Self {
            inner: Arc::new(SpscRingBuffer::new(min_capacity)),
        }
    }

    /// Actual capacity (power-of-2).
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Whether the queue is full.
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Try to push. Returns Err if full.
    pub fn try_push(&self, descriptor: Descriptor) -> Result<(), Descriptor> {
        self.inner.try_push(descriptor)
    }
}

impl QueueBackend for SpscQueue {
    fn push(&self, descriptor: Descriptor) {
        self.inner.push(descriptor);
    }

    fn try_pop(&self) -> Option<Descriptor> {
        self.inner.try_pop()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

// =============================================================================
// ShmSpscQueue — Cross-Process SPSC via Shared Memory + EventFd
// =============================================================================

/// Physical layout of an SPSC Queue in Shared Memory.
#[repr(C, align(128))]
pub struct ShmSpscLayout<T> {
    pub capacity: usize,
    pub mask: usize,
    pub tail: AtomicUsize, // Producer index
    pub head: AtomicUsize, // Consumer index
    _phantom: std::marker::PhantomData<T>,
}

/// SPSC Queue residing in Shared Memory with EventFd synchronization.
#[derive(Clone)]
pub struct ShmSpscQueue {
    /// Pointer to layout start in SHM.
    layout: *mut ShmSpscLayout<Descriptor>,
    /// Pointer to item buffer start in SHM.
    buffer: *mut UnsafeCell<MaybeUninit<Descriptor>>,
    /// File descriptor for signaling (Linux eventfd).
    signal_fd: Option<RawFd>,
}

// Safety: Access is governed by atomics in SHM.
unsafe impl Send for ShmSpscQueue {}
unsafe impl Sync for ShmSpscQueue {}

impl ShmSpscQueue {
    /// Create a new eventfd for signaling.
    pub fn create_eventfd() -> std::io::Result<RawFd> {
        eventfd(0, EfdFlags::EFD_CLOEXEC | EfdFlags::EFD_NONBLOCK)
            .map(|fd| fd.into_raw_fd())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    /// Create an ShmSpscQueue from raw pointers already allocated in SHM.
    ///
    /// # Safety
    /// Caller must guarantee the pointer is valid and sufficiently sized (struct + capacity * T).
    pub unsafe fn from_raw_parts(
        base_ptr: *mut u8,
        _capacity: usize,
        signal_fd: Option<RawFd>,
    ) -> Self {
        let layout = base_ptr as *mut ShmSpscLayout<Descriptor>;
        // Buffer starts after the struct (with 128-byte alignment)
        let buffer = base_ptr.add(std::mem::size_of::<ShmSpscLayout<Descriptor>>())
            as *mut UnsafeCell<MaybeUninit<Descriptor>>;

        Self {
            layout,
            buffer,
            signal_fd,
        }
    }

    /// Signal the consumer that new data is available.
    pub fn signal(&self) {
        if let Some(fd) = self.signal_fd {
            let buf = 1u64.to_ne_bytes();
            let _ = nix::unistd::write(fd, &buf);
        }
    }

    /// Wait for a signal from the producer (blocking).
    pub fn wait(&self) {
        if let Some(fd) = self.signal_fd {
            let mut buf = [0u8; 8];
            let _ = nix::unistd::read(fd, &mut buf);
        }
    }

    pub fn try_push(&self, item: Descriptor) -> Result<(), Descriptor> {
        // SAFETY: self.layout points to valid SHM-mapped ShmSpscLayout, allocated during construction.
        unsafe {
            let tail = (*self.layout).tail.load(Ordering::Relaxed);
            let head = (*self.layout).head.load(Ordering::Acquire);
            let capacity = (*self.layout).capacity;
            let mask = (*self.layout).mask;

            if tail.wrapping_sub(head) >= capacity {
                return Err(item);
            }

            let idx = tail & mask;
            let slot_ptr = self.buffer.add(idx);
            (*(*slot_ptr).get()).write(item);

            (*self.layout)
                .tail
                .store(tail.wrapping_add(1), Ordering::Release);
            self.signal();
            Ok(())
        }
    }

    pub fn try_pop(&self) -> Option<Descriptor> {
        // SAFETY: self.layout points to valid SHM-mapped ShmSpscLayout, allocated during construction.
        unsafe {
            let head = (*self.layout).head.load(Ordering::Relaxed);
            let tail = (*self.layout).tail.load(Ordering::Acquire);
            let mask = (*self.layout).mask;

            if head == tail {
                return None;
            }

            let idx = head & mask;
            let slot_ptr = self.buffer.add(idx);
            let item = (*(*slot_ptr).get()).assume_init_read();

            (*self.layout)
                .head
                .store(head.wrapping_add(1), Ordering::Release);
            Some(item)
        }
    }
}

impl QueueBackend for ShmSpscQueue {
    fn push(&self, descriptor: Descriptor) {
        let mut item = descriptor;
        loop {
            match self.try_push(item) {
                Ok(()) => return,
                Err(returned) => {
                    item = returned;
                    std::hint::spin_loop();
                }
            }
        }
    }

    fn try_pop(&self) -> Option<Descriptor> {
        self.try_pop()
    }

    fn len(&self) -> usize {
        // SAFETY: self.layout points to valid SHM-mapped ShmSpscLayout, allocated during construction.
        unsafe {
            let tail = (*self.layout).tail.load(Ordering::Acquire);
            let head = (*self.layout).head.load(Ordering::Acquire);
            tail.wrapping_sub(head)
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use vil_types::{HostId, PortId, SampleId};

    fn make_desc(id: u64) -> Descriptor {
        Descriptor {
            sample_id: SampleId(id),
            origin_host: HostId(0),
            origin_port: PortId(1),
            lineage_id: id * 10,
            publish_ts: 0,
        }
    }

    // --- SpscRingBuffer tests ---

    #[test]
    fn test_capacity_rounds_up() {
        let rb = SpscRingBuffer::<u64>::new(3);
        assert_eq!(rb.capacity(), 4); // next power of 2

        let rb = SpscRingBuffer::<u64>::new(8);
        assert_eq!(rb.capacity(), 8); // already power of 2

        let rb = SpscRingBuffer::<u64>::new(1);
        assert_eq!(rb.capacity(), 1);
    }

    #[test]
    #[should_panic(expected = "SPSC capacity must be > 0")]
    fn test_zero_capacity_panics() {
        let _ = SpscRingBuffer::<u64>::new(0);
    }

    #[test]
    fn test_push_pop_single() {
        let rb = SpscRingBuffer::new(4);
        assert!(rb.is_empty());

        rb.try_push(42u64).unwrap();
        assert_eq!(rb.len(), 1);
        assert!(!rb.is_empty());

        let val = rb.try_pop().unwrap();
        assert_eq!(val, 42);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_fifo_ordering() {
        let rb = SpscRingBuffer::new(8);
        for i in 0..8 {
            rb.try_push(i as u64).unwrap();
        }
        assert!(rb.is_full());

        for i in 0..8 {
            assert_eq!(rb.try_pop().unwrap(), i as u64);
        }
        assert!(rb.is_empty());
    }

    #[test]
    fn test_full_returns_err() {
        let rb = SpscRingBuffer::new(2);
        rb.try_push(1u64).unwrap();
        rb.try_push(2u64).unwrap();

        let result = rb.try_push(3u64);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 3);
    }

    #[test]
    fn test_empty_returns_none() {
        let rb = SpscRingBuffer::<u64>::new(4);
        assert!(rb.try_pop().is_none());
    }

    #[test]
    fn test_wrap_around() {
        let rb = SpscRingBuffer::new(4);

        // Fill and drain multiple times to exercise wrap-around
        for round in 0..5 {
            for i in 0..4 {
                rb.try_push(round * 10 + i).unwrap();
            }
            for i in 0..4 {
                assert_eq!(rb.try_pop().unwrap(), round * 10 + i);
            }
            assert!(rb.is_empty());
        }
    }

    #[test]
    fn test_interleaved_push_pop() {
        let rb = SpscRingBuffer::new(4);

        rb.try_push(1u64).unwrap();
        rb.try_push(2u64).unwrap();
        assert_eq!(rb.try_pop().unwrap(), 1);

        rb.try_push(3u64).unwrap();
        assert_eq!(rb.try_pop().unwrap(), 2);
        assert_eq!(rb.try_pop().unwrap(), 3);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_drop_drains_remaining() {
        use std::sync::atomic::AtomicUsize;

        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug)]
        struct Counted(#[allow(dead_code)] u64);
        impl Drop for Counted {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        DROP_COUNT.store(0, Ordering::Relaxed);

        {
            let rb = SpscRingBuffer::new(8);
            rb.try_push(Counted(1)).unwrap();
            rb.try_push(Counted(2)).unwrap();
            rb.try_push(Counted(3)).unwrap();
            // Drop with 3 items still in queue
        }

        assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 3);
    }

    // --- SpscQueue (Descriptor) tests ---

    #[test]
    fn test_spsc_queue_descriptor() {
        let q = SpscQueue::new(64);
        assert!(q.is_empty());
        assert_eq!(q.capacity(), 64);

        q.push(make_desc(1));
        q.push(make_desc(2));
        assert_eq!(q.len(), 2);

        let d1 = q.try_pop().unwrap();
        assert_eq!(d1.sample_id, SampleId(1));

        let d2 = q.try_pop().unwrap();
        assert_eq!(d2.sample_id, SampleId(2));
        assert!(q.is_empty());
    }

    #[test]
    fn test_spsc_queue_backend_trait() {
        let q: Box<dyn QueueBackend> = Box::new(SpscQueue::new(16));
        q.push(make_desc(42));
        assert_eq!(q.len(), 1);
        let d = q.try_pop().unwrap();
        assert_eq!(d.sample_id, SampleId(42));
    }

    #[test]
    fn test_spsc_queue_full() {
        let q = SpscQueue::new(2); // capacity = 2
        assert!(q.try_push(make_desc(1)).is_ok());
        assert!(q.try_push(make_desc(2)).is_ok());
        assert!(q.try_push(make_desc(3)).is_err());
        assert!(q.is_full());
    }

    // --- Cross-thread stress test ---

    #[test]
    fn test_cross_thread_stress() {
        let count = 100_000u64;
        let rb = Arc::new(SpscRingBuffer::new(1024));

        let producer_rb = rb.clone();
        let producer = thread::spawn(move || {
            for i in 0..count {
                producer_rb.push(i);
            }
        });

        let consumer_rb = rb.clone();
        let consumer = thread::spawn(move || {
            let mut received = Vec::with_capacity(count as usize);
            while received.len() < count as usize {
                if let Some(val) = consumer_rb.try_pop() {
                    received.push(val);
                } else {
                    std::hint::spin_loop();
                }
            }
            received
        });

        producer.join().unwrap();
        let received = consumer.join().unwrap();

        // Verify FIFO ordering
        assert_eq!(received.len(), count as usize);
        for (i, val) in received.iter().enumerate() {
            assert_eq!(*val, i as u64, "FIFO violation at index {}", i);
        }
    }

    #[test]
    fn test_cross_thread_descriptor_stress() {
        let count = 50_000u64;
        let q = SpscQueue::new(512);

        let producer_q = q.clone();
        let producer = thread::spawn(move || {
            for i in 0..count {
                producer_q.push(make_desc(i));
            }
        });

        let consumer_q = q.clone();
        let consumer = thread::spawn(move || {
            let mut received = Vec::with_capacity(count as usize);
            while received.len() < count as usize {
                if let Some(d) = consumer_q.try_pop() {
                    received.push(d);
                } else {
                    std::hint::spin_loop();
                }
            }
            received
        });

        producer.join().unwrap();
        let received = consumer.join().unwrap();

        assert_eq!(received.len(), count as usize);
        for (i, d) in received.iter().enumerate() {
            assert_eq!(d.sample_id, SampleId(i as u64), "FIFO violation at {}", i);
        }
    }

    #[test]
    fn test_shm_spsc_queue_basic() {
        let capacity = 16;
        let size = std::mem::size_of::<ShmSpscLayout<Descriptor>>()
            + capacity * std::mem::size_of::<UnsafeCell<MaybeUninit<Descriptor>>>();

        // Simulate SHM with a manually aligned Vec
        let mut storage = vec![0u8; size + 256];
        let raw_ptr = storage.as_mut_ptr();
        // SAFETY: raw_ptr is from valid allocation, alignment adjustment ensures cache-line alignment.
        let base_ptr = unsafe {
            let offset = (128 - (raw_ptr as usize % 128)) % 128;
            raw_ptr.add(offset)
        };

        // SAFETY: raw_ptr is from valid allocation, alignment adjustment ensures cache-line alignment.
        unsafe {
            let layout = base_ptr as *mut ShmSpscLayout<Descriptor>;
            (*layout).capacity = capacity;
            (*layout).mask = capacity - 1;
            (*layout).tail.store(0, Ordering::Release);
            (*layout).head.store(0, Ordering::Release);

            let efd = ShmSpscQueue::create_eventfd().unwrap();
            let q1 = ShmSpscQueue::from_raw_parts(base_ptr, capacity, Some(efd));
            let q2 = ShmSpscQueue::from_raw_parts(base_ptr, capacity, Some(efd));

            let desc = make_desc(123);
            q1.try_push(desc).unwrap();

            assert_eq!(q2.len(), 1);
            let popped = q2.try_pop().unwrap();
            assert_eq!(popped.sample_id, SampleId(123));

            // Clean up eventfd
            nix::unistd::close(efd).unwrap();
        }
    }
}
