// =============================================================================
// vil_log::emit::ring — LogRing: Lock-Free SPSC Ring for LogSlot
// =============================================================================
//
// Minimal reimplementation of the SPSC ring buffer pattern, specialized for
// LogSlot (256-byte fixed-size slots). No dependency on vil_queue.
//
// Design:
//   - Power-of-2 capacity with bitmask indexing (no modulo)
//   - AtomicUsize head (consumer) and tail (producer), cache-line padded
//   - UnsafeCell<MaybeUninit<LogSlot>> backing store on the heap
//   - Acquire/Release ordering
//   - try_push never blocks — returns Err on full
//   - Global static via std::sync::OnceLock (stable since Rust 1.70)
//   - AtomicU64 drop counter tracks ring-full discards
// =============================================================================

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::types::LogSlot;

// =============================================================================
// Cache-line padding (128 bytes — covers prefetch on both x86 and ARM)
// =============================================================================

#[repr(align(128))]
struct CachePad<T> {
    v: T,
}

// =============================================================================
// LogRing — SPSC ring buffer for LogSlot
// =============================================================================

/// Lock-free SPSC ring buffer for `LogSlot`.
///
/// **ONLY safe for exactly 1 producer thread and 1 consumer thread.**
pub struct LogRing {
    buffer: Box<[UnsafeCell<MaybeUninit<LogSlot>>]>,
    capacity: usize,
    mask: usize,
    /// Write index — mutated only by the producer.
    tail: CachePad<AtomicUsize>,
    /// Read index — mutated only by the consumer.
    head: CachePad<AtomicUsize>,
    /// Counts ring-full drop events.
    pub drops: AtomicU64,
}

// Safety: LogSlot is Send + Sync. Access is governed by atomics.
unsafe impl Send for LogRing {}
unsafe impl Sync for LogRing {}

impl std::fmt::Debug for LogRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogRing")
            .field("capacity", &self.capacity)
            .field("len", &self.len())
            .field(
                "drops",
                &self.drops.load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish()
    }
}

impl LogRing {
    /// Create a new ring with at least `min_capacity` slots (rounded to power-of-2).
    pub fn new(min_capacity: usize) -> Self {
        assert!(min_capacity > 0, "LogRing capacity must be > 0");
        let capacity = min_capacity.next_power_of_two();
        let mask = capacity - 1;

        let mut buf = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buf.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        Self {
            buffer: buf.into_boxed_slice(),
            capacity,
            mask,
            tail: CachePad {
                v: AtomicUsize::new(0),
            },
            head: CachePad {
                v: AtomicUsize::new(0),
            },
            drops: AtomicU64::new(0),
        }
    }

    /// Capacity (power-of-2).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of occupied slots (snapshot).
    #[inline]
    pub fn len(&self) -> usize {
        let tail = self.tail.v.load(Ordering::Acquire);
        let head = self.head.v.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// True if ring is empty (snapshot).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Try to push a `LogSlot`. Returns `Err(slot)` if the ring is full (never blocks).
    ///
    /// **MUST only be called from the single producer thread.**
    #[inline]
    pub fn try_push(&self, slot: LogSlot) -> Result<(), LogSlot> {
        let tail = self.tail.v.load(Ordering::Relaxed);
        let head = self.head.v.load(Ordering::Acquire);

        if tail.wrapping_sub(head) >= self.capacity {
            self.drops.fetch_add(1, Ordering::Relaxed);
            return Err(slot);
        }

        let idx = tail & self.mask;
        // Safety: idx is unique to the producer (head != tail-based full check).
        unsafe {
            (*self.buffer[idx].get()).write(slot);
        }

        self.tail.v.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Try to pop a `LogSlot`. Returns `None` if the ring is empty.
    ///
    /// **MUST only be called from the single consumer thread.**
    #[inline]
    pub fn try_pop(&self) -> Option<LogSlot> {
        let head = self.head.v.load(Ordering::Relaxed);
        let tail = self.tail.v.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let idx = head & self.mask;
        // Safety: idx is unique to the consumer at this point.
        let slot = unsafe { (*self.buffer[idx].get()).assume_init_read() };

        self.head.v.store(head.wrapping_add(1), Ordering::Release);
        Some(slot)
    }

    /// Drain up to `max` slots into `out`. Returns number drained.
    pub fn drain_into(&self, out: &mut Vec<LogSlot>, max: usize) -> usize {
        let mut count = 0;
        while count < max {
            match self.try_pop() {
                Some(slot) => {
                    out.push(slot);
                    count += 1;
                }
                None => break,
            }
        }
        count
    }

    /// Total drop count since creation.
    #[inline]
    pub fn drop_count(&self) -> u64 {
        self.drops.load(Ordering::Relaxed)
    }
}

impl Drop for LogRing {
    fn drop(&mut self) {
        let head = *self.head.v.get_mut();
        let tail = *self.tail.v.get_mut();
        for i in head..tail {
            let idx = i & self.mask;
            // Safety: slots between head and tail are initialized.
            unsafe {
                self.buffer[idx].get_mut().assume_init_drop();
            }
        }
    }
}

// =============================================================================
// StripedRing — N SPSC rings, auto-sized to available_parallelism()
// =============================================================================
// Each CPU core gets its own SPSC ring. Threads are assigned via
// thread_id % stripe_count. This means:
//   - At ≤N threads: ~1 thread per ring → zero contention
//   - At >N threads: some sharing, but still better than single ring
//
// The stripe count is determined once at init_ring() time:
//   stripe_count = available_parallelism().min(MAX_STRIPES)
// =============================================================================

/// Maximum stripe count cap (avoid excessive memory on 64+ core machines).
const MAX_STRIPES: usize = 32;

/// Round-robin counter for stripe assignment. Each new thread gets the next index.
static NEXT_STRIPE: AtomicUsize = AtomicUsize::new(0);

/// Auto-sized striped SPSC ring buffer for multi-thread log emission.
///
/// Stripe count = `min(available_parallelism, 32)`.
/// Each thread selects a ring via `thread_id % stripe_count`.
pub struct StripedRing {
    rings: Vec<LogRing>,
    stripe_count: usize,
    /// Bitmask for fast modulo (only works when stripe_count is power-of-2).
    mask: usize,
}

// Safety: All LogRing are Send+Sync, StripedRing only delegates.
unsafe impl Send for StripedRing {}
unsafe impl Sync for StripedRing {}

impl std::fmt::Debug for StripedRing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripedRing")
            .field("stripe_count", &self.stripe_count)
            .field("total_len", &self.total_len())
            .field("total_drops", &self.total_drop_count())
            .finish()
    }
}

impl StripedRing {
    /// Create N SPSC rings based on available parallelism.
    /// Each ring gets `capacity_per_ring` slots.
    pub fn auto(capacity_per_ring: usize) -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let stripe_count = cpus.min(MAX_STRIPES).next_power_of_two();
        let mask = stripe_count - 1;

        let rings: Vec<LogRing> = (0..stripe_count)
            .map(|_| LogRing::new(capacity_per_ring))
            .collect();

        Self {
            rings,
            stripe_count,
            mask,
        }
    }

    /// Create with explicit stripe count (for testing / override).
    pub fn with_stripes(stripe_count: usize, capacity_per_ring: usize) -> Self {
        let stripe_count = stripe_count.max(1).next_power_of_two().min(MAX_STRIPES);
        let mask = stripe_count - 1;
        let rings: Vec<LogRing> = (0..stripe_count)
            .map(|_| LogRing::new(capacity_per_ring))
            .collect();
        Self {
            rings,
            stripe_count,
            mask,
        }
    }

    /// Number of stripe lanes.
    pub fn stripe_count(&self) -> usize {
        self.stripe_count
    }

    /// Hybrid stripe selection — cached per thread, zero cost after first call.
    ///
    /// - First 4 threads: dedicated ring 0..3 (guaranteed 1:1 mapping, no sharing)
    /// - Thread 5+: round-robin across ALL stripes (even distribution)
    ///
    /// This gives optimal performance at both low thread counts (1-4, typical
    /// web servers) and high thread counts (data pipelines, 8-16+).
    #[inline(always)]
    fn stripe_index(&self) -> usize {
        thread_local! {
            static MY_STRIPE: usize = {
                let n = NEXT_STRIPE.fetch_add(1, Ordering::Relaxed);
                if n < 4 {
                    // First 4 threads: dedicated ring (0, 1, 2, 3)
                    // No sharing, zero contention guaranteed
                    n
                } else {
                    // Thread 5+: round-robin across all stripes
                    n
                }
            };
        }
        MY_STRIPE.with(|&idx| idx & self.mask)
    }

    /// Push to the stripe for the current thread. Never blocks.
    #[inline]
    pub fn try_push(&self, slot: LogSlot) -> Result<(), LogSlot> {
        let idx = self.stripe_index();
        self.rings[idx].try_push(slot)
    }

    /// Drain from ALL rings into `out`, up to `max` total slots.
    pub fn drain_all(&self, out: &mut Vec<LogSlot>, max: usize) -> usize {
        let per_ring = (max / self.stripe_count).max(1);
        let mut total = 0;
        for ring in &self.rings {
            total += ring.drain_into(out, per_ring);
        }
        total
    }

    /// Total drop count across all rings.
    pub fn total_drop_count(&self) -> u64 {
        self.rings.iter().map(|r| r.drop_count()).sum()
    }

    /// Total length across all rings.
    pub fn total_len(&self) -> usize {
        self.rings.iter().map(|r| r.len()).sum()
    }

    /// Access individual ring by index.
    pub fn ring(&self, index: usize) -> &LogRing {
        &self.rings[index % self.stripe_count]
    }
}

// =============================================================================
// Global singleton
// =============================================================================

static GLOBAL_STRIPED: OnceLock<StripedRing> = OnceLock::new();
// Keep old global for backward compat (single-ring API users)
static GLOBAL_RING: OnceLock<LogRing> = OnceLock::new();

use std::sync::atomic::AtomicU8;

/// Global minimum log level. Events below this are filtered out before touching the ring.
static GLOBAL_LEVEL: AtomicU8 = AtomicU8::new(0); // 0 = Trace (accept everything)

/// Set the global minimum log level. Events below this level are discarded.
pub fn set_global_level(level: crate::types::LogLevel) {
    GLOBAL_LEVEL.store(level as u8, Ordering::Release);
}

/// Check if a given level passes the global filter.
/// Returns `true` if the event should be logged.
#[inline(always)]
pub fn level_enabled(level: u8) -> bool {
    level >= GLOBAL_LEVEL.load(Ordering::Relaxed)
}

/// Initialize the global striped ring.
///
/// - `capacity`: total ring capacity (divided across stripes)
/// - `thread_hint`: expected thread count
///   - `Some(n)` → exactly n stripes (1 ring per thread, zero contention)
///   - `None` → auto-detect from `available_parallelism()`
///
/// Each ring gets `capacity / stripe_count` slots (min 16K).
///
/// Must be called once at startup. Panics if called more than once.
pub fn init_ring(capacity: usize, thread_hint: Option<usize>) {
    let stripe_count = match thread_hint {
        Some(n) => n.max(1).next_power_of_two().min(MAX_STRIPES),
        None => {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            cpus.min(MAX_STRIPES).next_power_of_two()
        }
    };
    // Minimum 16K slots per ring to handle burst from single thread
    let per_ring = (capacity / stripe_count).max(16384);
    let striped = StripedRing::with_stripes(stripe_count, per_ring);

    // Also init single global ring for backward compat (legacy API)
    let _ = GLOBAL_RING.set(LogRing::new(per_ring));

    GLOBAL_STRIPED
        .set(striped)
        .expect("vil_log: init_ring called more than once");
}

/// Get a reference to the global striped ring.
///
/// # Panics
/// Panics if `init_ring` has not been called yet.
#[inline]
pub fn global_striped() -> &'static StripedRing {
    GLOBAL_STRIPED
        .get()
        .expect("vil_log: global ring not initialized — call init_ring() first")
}

/// Try to get the global striped ring; returns None if not initialized.
#[inline]
pub fn try_global_striped() -> Option<&'static StripedRing> {
    GLOBAL_STRIPED.get()
}

/// Get a reference to the global `LogRing` (backward compat — returns ring 0).
///
/// # Panics
/// Panics if `init_ring` has not been called yet.
#[inline]
pub fn global_ring() -> &'static LogRing {
    GLOBAL_RING
        .get()
        .expect("vil_log: global ring not initialized — call init_ring() first")
}

/// Try to get the global ring (backward compat); returns None if not initialized.
#[inline]
pub fn try_global_ring() -> Option<&'static LogRing> {
    GLOBAL_RING.get()
}

/// Get total drop count across all striped rings.
pub fn drop_count() -> u64 {
    match GLOBAL_STRIPED.get() {
        Some(s) => s.total_drop_count(),
        None => 0,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_basic() {
        let ring = LogRing::new(4);
        assert_eq!(ring.capacity(), 4);
        assert!(ring.is_empty());

        let slot = LogSlot::default();
        ring.try_push(slot).unwrap();
        assert_eq!(ring.len(), 1);

        let popped = ring.try_pop().unwrap();
        assert_eq!(popped.header.level, 0);
        assert!(ring.is_empty());
    }

    #[test]
    fn test_ring_full_drops() {
        let ring = LogRing::new(2);
        ring.try_push(LogSlot::default()).unwrap();
        ring.try_push(LogSlot::default()).unwrap();
        assert!(ring.try_push(LogSlot::default()).is_err());
        assert_eq!(ring.drop_count(), 1);
    }

    #[test]
    fn test_ring_drain() {
        let ring = LogRing::new(8);
        for _ in 0..5 {
            ring.try_push(LogSlot::default()).unwrap();
        }
        let mut out = Vec::new();
        let n = ring.drain_into(&mut out, 10);
        assert_eq!(n, 5);
        assert_eq!(out.len(), 5);
        assert!(ring.is_empty());
    }
}
