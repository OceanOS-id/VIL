// =============================================================================
// vil_shm::allocator — Bump Allocator for Exchange Heap Regions
// =============================================================================
// Simple but efficient bump allocator for allocation within shared
// exchange heap regions. Each region has one BumpAllocator.
//
// Characteristics:
//   - O(1) allocation (advance pointer only)
//   - Alignment-aware (pad to alignment of T)
//   - Thread-safe via AtomicUsize cursor
//   - No individual free — only whole-region reset
//   - Suited for hot-path zero-copy: fast alloc, batch reclaim
//
// TASK LIST:
// [x] BumpAllocator — atomic bump allocation
// [x] alloc — aligned slot allocation, returns Offset
// [x] try_alloc — non-panicking version
// [x] reset — reset cursor to start (batch reclaim)
// [x] used / remaining / capacity
// [x] Unit tests
// [ ] TODO(future): SlabAllocator for fixed-size hot path
// [ ] TODO(future): FreeList allocator for variable-size
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::offset::Offset;

/// Bump allocator for exchange heap regions.
///
/// Thread-safe (AtomicUsize cursor). O(1) allocation.
/// No individual free — reset the entire allocator at once.
///
/// Layout: `[--- used ---> cursor ... remaining ... capacity]`
pub struct BumpAllocator {
    /// Next allocation position (in bytes from region start).
    cursor: AtomicUsize,
    /// Total region capacity in bytes.
    capacity: usize,
}

impl BumpAllocator {
    /// Create a new allocator for a region with `capacity` bytes.
    pub fn new(capacity: usize) -> Self {
        Self {
            cursor: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Try to allocate `size` bytes with alignment `align`.
    /// Returns `Some(Offset)` on success, `None` if full.
    ///
    /// # Panics
    /// Panics if `align` is not a power of 2.
    pub fn try_alloc(&self, size: usize, align: usize) -> Option<Offset> {
        assert!(align.is_power_of_two(), "alignment must be power of 2");

        loop {
            let current = self.cursor.load(Ordering::Relaxed);

            // Pad cursor to alignment
            let aligned = (current + align - 1) & !(align - 1);

            // Check if it fits
            let new_cursor = aligned + size;
            if new_cursor > self.capacity {
                return None; // Region full
            }

            // CAS for thread-safety
            match self.cursor.compare_exchange_weak(
                current,
                new_cursor,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Offset::new(aligned as u64)),
                Err(_) => continue, // Retry (concurrent alloc)
            }
        }
    }

    /// Allocate a slot for type T. Returns Offset on success.
    pub fn try_alloc_typed<T>(&self) -> Option<Offset> {
        self.try_alloc(std::mem::size_of::<T>(), std::mem::align_of::<T>())
    }

    /// Allocate a slot. Panics if region is full.
    pub fn alloc(&self, size: usize, align: usize) -> Offset {
        self.try_alloc(size, align)
            .expect("exchange heap region exhausted")
    }

    /// Allocate a slot for type T. Panics if region is full.
    pub fn alloc_typed<T>(&self) -> Offset {
        self.alloc(std::mem::size_of::<T>(), std::mem::align_of::<T>())
    }

    /// Reset allocator — all previous allocations become invalid.
    /// Used for batch reclaim.
    ///
    /// # Safety
    /// Caller must ensure no live references to previously allocated data.
    pub fn reset(&self) {
        self.cursor.store(0, Ordering::Release);
    }

    /// Bytes already allocated.
    pub fn used(&self) -> usize {
        self.cursor.load(Ordering::Acquire)
    }

    /// Bytes still available (approximate — may change due to alignment).
    pub fn remaining(&self) -> usize {
        self.capacity.saturating_sub(self.used())
    }

    /// Total region capacity (bytes).
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_alloc() {
        let alloc = BumpAllocator::new(1024);
        let offset = alloc.alloc(64, 8);
        assert_eq!(offset, Offset::new(0));
        assert_eq!(alloc.used(), 64);
    }

    #[test]
    fn test_alignment() {
        let alloc = BumpAllocator::new(1024);

        // Alloc 1 byte, then alloc with align=8
        let o1 = alloc.alloc(1, 1);
        assert_eq!(o1, Offset::new(0));

        let o2 = alloc.alloc(8, 8);
        assert_eq!(o2, Offset::new(8)); // Padded to align 8
    }

    #[test]
    fn test_typed_alloc() {
        let alloc = BumpAllocator::new(1024);

        let o1 = alloc.alloc_typed::<u8>();
        assert_eq!(o1, Offset::new(0));

        let o2 = alloc.alloc_typed::<u64>();
        // u64 needs align 8, so should be padded
        assert_eq!(o2.as_u64() % 8, 0);
    }

    #[test]
    fn test_try_alloc_returns_none_when_full() {
        let alloc = BumpAllocator::new(16);

        // Should succeed
        assert!(alloc.try_alloc(16, 1).is_some());

        // Should fail — no space left
        assert!(alloc.try_alloc(1, 1).is_none());
    }

    #[test]
    fn test_reset() {
        let alloc = BumpAllocator::new(64);
        alloc.alloc(32, 1);
        assert_eq!(alloc.used(), 32);

        alloc.reset();
        assert_eq!(alloc.used(), 0);
        assert_eq!(alloc.remaining(), 64);

        // Can alloc again after reset
        let o = alloc.alloc(64, 1);
        assert_eq!(o, Offset::new(0));
    }

    #[test]
    fn test_remaining_and_capacity() {
        let alloc = BumpAllocator::new(256);
        assert_eq!(alloc.capacity(), 256);
        assert_eq!(alloc.remaining(), 256);

        alloc.alloc(100, 1);
        assert_eq!(alloc.remaining(), 156);
    }

    #[test]
    #[should_panic(expected = "alignment must be power of 2")]
    fn test_bad_alignment_panics() {
        let alloc = BumpAllocator::new(64);
        alloc.alloc(8, 3); // 3 is not power of 2
    }

    #[test]
    fn test_concurrent_alloc() {
        use std::sync::Arc;
        use std::thread;

        let alloc = Arc::new(BumpAllocator::new(1_000_000));
        let mut handles = vec![];

        for _ in 0..8 {
            let a = alloc.clone();
            handles.push(thread::spawn(move || {
                let mut offsets = vec![];
                for _ in 0..1000 {
                    if let Some(o) = a.try_alloc(64, 8) {
                        offsets.push(o);
                    }
                }
                offsets
            }));
        }

        let mut all_offsets = vec![];
        for h in handles {
            all_offsets.extend(h.join().unwrap());
        }

        // Verify no overlapping allocations
        all_offsets.sort_by_key(|o| o.as_u64());
        for window in all_offsets.windows(2) {
            // Each alloc is 64 bytes, so next should be at least 64 apart
            assert!(
                window[1].as_u64() >= window[0].as_u64() + 64,
                "Overlap detected: {} and {}",
                window[0],
                window[1]
            );
        }
    }
}
