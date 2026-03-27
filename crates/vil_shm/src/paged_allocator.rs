// =============================================================================
// vil_shm::paged_allocator — Paged Bump Allocator for Adaptive Compaction
// =============================================================================
// PagedAllocator divides a region into small blocks (Pages).
// This allows moving data between blocks for defragmentation
// without resetting the entire region.
//
// TASK LIST:
// [x] Page — smallest allocation unit (bump-based)
// [x] PagedAllocator — management of a list of Pages
// [x] try_alloc — cross-page allocation
// [x] Unit tests
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use crate::offset::Offset;

pub const PAGE_SIZE: usize = 1024 * 1024; // 1MB default page size

/// A single allocation unit within a region.
pub struct Page {
    pub start_offset: usize,
    pub cursor: AtomicUsize,
}

impl Page {
    #[doc(alias = "vil_keep")]
    pub fn new(start_offset: usize) -> Self {
        Self {
            start_offset,
            cursor: AtomicUsize::new(0),
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn try_alloc(&self, size: usize, align: usize) -> Option<usize> {
        loop {
            let current = self.cursor.load(Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_cursor = aligned + size;

            if new_cursor > PAGE_SIZE {
                return None;
            }

            match self.cursor.compare_exchange_weak(
                current,
                new_cursor,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(self.start_offset + aligned),
                Err(_) => continue,
            }
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn used(&self) -> usize {
        self.cursor.load(Ordering::Acquire)
    }
}

/// Allocator managing a collection of Pages within a single region buffer.
pub struct PagedAllocator {
    pub pages: Vec<Page>,
    pub active_page_idx: AtomicUsize,
    pub total_capacity: usize,
}

impl PagedAllocator {
    #[doc(alias = "vil_keep")]
    pub fn new(total_capacity: usize) -> Self {
        let num_pages = total_capacity / PAGE_SIZE;
        let mut pages = Vec::with_capacity(num_pages);
        for i in 0..num_pages {
            pages.push(Page::new(i * PAGE_SIZE));
        }

        Self {
            pages,
            active_page_idx: AtomicUsize::new(0),
            total_capacity,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn try_alloc(&self, size: usize, align: usize) -> Option<Offset> {
        if size > PAGE_SIZE {
            return None; // Cannot fit in a single page
        }

        let mut idx = self.active_page_idx.load(Ordering::Relaxed);

        while idx < self.pages.len() {
            if let Some(abs_offset) = self.pages[idx].try_alloc(size, align) {
                return Some(Offset::new(abs_offset as u64));
            }

            // Current page full, move to next
            let next_idx = idx + 1;
            if next_idx >= self.pages.len() {
                return None;
            }

            // CAS to advance the active page index
            match self.active_page_idx.compare_exchange(
                idx,
                next_idx,
                Ordering::SeqCst,
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    idx = next_idx;
                }
                Err(latest) => {
                    idx = latest;
                }
            }
        }

        None
    }

    #[doc(alias = "vil_keep")]
    pub fn used(&self) -> usize {
        self.pages.iter().map(|p| p.used()).sum()
    }

    #[doc(alias = "vil_keep")]
    pub fn remaining(&self) -> usize {
        self.total_capacity.saturating_sub(self.used())
    }

    #[doc(alias = "vil_keep")]
    pub fn reset(&self) {
        for page in &self.pages {
            page.cursor.store(0, Ordering::Release);
        }
        self.active_page_idx.store(0, Ordering::Release);
    }

    #[doc(alias = "vil_keep")]
    pub fn reset_to(&self, next_offset: usize) {
        let page_idx = next_offset / PAGE_SIZE;
        let page_offset = next_offset % PAGE_SIZE;

        for i in 0..self.pages.len() {
            if i < page_idx {
                // Mark previous pages as completely full
                self.pages[i].cursor.store(PAGE_SIZE, Ordering::Release);
            } else if i == page_idx {
                // Set current page cursor to the packed offset
                self.pages[i].cursor.store(page_offset, Ordering::Release);
            } else {
                // Future pages are empty
                self.pages[i].cursor.store(0, Ordering::Release);
            }
        }
        self.active_page_idx.store(page_idx.min(self.pages.len() - 1), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paged_alloc_basic() {
        let alloc = PagedAllocator::new(PAGE_SIZE * 2);
        let o1 = alloc.try_alloc(100, 8).unwrap();
        assert_eq!(o1.as_u64(), 0);

        // Fill the rest of page 0
        alloc.try_alloc(PAGE_SIZE - 100, 1).unwrap();

        // This should go to page 1
        let o2 = alloc.try_alloc(100, 8).unwrap();
        assert_eq!(o2.as_u64(), PAGE_SIZE as u64);
    }

    #[test]
    fn test_paged_alloc_full() {
        let alloc = PagedAllocator::new(PAGE_SIZE);
        alloc.try_alloc(PAGE_SIZE, 1).unwrap();
        assert!(alloc.try_alloc(1, 1).is_none());
    }
}
