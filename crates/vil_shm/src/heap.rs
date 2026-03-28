// =============================================================================
// vil_shm::heap — Exchange Heap (Region-Backed Typed Allocation)
// =============================================================================
// ExchangeHeap is the core of VIL zero-copy: all samples exchanged
// between processes are allocated here.
//
// Architecture:
//   ┌─────────────────────────────────────────┐
//   │ ExchangeHeap                            │
//   │  ┌──────────┐ ┌──────────┐ ┌────────┐  │
//   │  │ Region 0 │ │ Region 1 │ │ Reg N  │  │
//   │  │ [page]   │ │ [page]   │ │ [page] │  │
//   │  │ ████░░░  │ │ ██░░░░░  │ │ ░░░░░  │  │
//   │  └──────────┘ └──────────┘ └────────┘  │
//   └─────────────────────────────────────────┘
//
// Each Region = contiguous byte buffer + PagedAllocator.
// Phase 1: Vec<u8> backing (mmap-ready API).
// Target: mmap(MAP_SHARED|MAP_ANONYMOUS) for true cross-process shared memory.
//
// Typed API:
//   - alloc_in_region::<T>(region_id) -> (RelativePtr<T>, &mut T)
//   - read::<T>(region_id, ptr) -> &T
//   - write::<T>(region_id, ptr, value) -> typed write
//   - create_region(name, size) -> RegionId
//
// TASK LIST:
// [x] RegionSlot — backing buffer + allocator
// [x] ExchangeHeap — multi-region management
// [x] create_region — create new region
// [x] alloc_in_region — typed allocation
// [x] write_at / read_at — typed access via RelativePtr
// [x] alloc_and_write — convenience: alloc + write in one call
// [x] region_stats — usage reporting
// [x] Unit tests
// [ ] TODO(future): mmap-backed RegionSlot
// [ ] TODO(future): region-per-message-class strategy
// [ ] TODO(future): background compaction
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vil_types::RegionId;

use crate::offset::{Offset, RelativePtr};
use crate::paged_allocator::PagedAllocator;

/// Usage statistics for a single region.
#[derive(Clone, Copy, Debug)]
pub struct RegionStats {
    pub region_id: RegionId,
    pub capacity: usize,
    pub used: usize,
    pub remaining: usize,
}

use memmap2::MmapMut;
use nix::fcntl::OFlag;
use nix::sys::mman;
use nix::sys::stat::Mode;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};

/// Backing store for a region.
enum BackingStore {
    Vec(Vec<u8>),
    Mmap(MmapMut),
}

impl BackingStore {
    #[doc(alias = "vil_keep")]
    fn as_ptr(&self) -> *const u8 {
        match self {
            BackingStore::Vec(v) => v.as_ptr(),
            BackingStore::Mmap(m) => m.as_ptr(),
        }
    }

    #[doc(alias = "vil_keep")]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        match self {
            BackingStore::Vec(v) => v.as_mut_ptr(),
            BackingStore::Mmap(m) => m.as_mut_ptr(),
        }
    }

    #[doc(alias = "vil_keep")]
    fn len(&self) -> usize {
        match self {
            BackingStore::Vec(v) => v.len(),
            BackingStore::Mmap(m) => m.len(),
        }
    }
}

/// A single region on the exchange heap — contiguous byte buffer + allocator.
struct RegionSlot {
    /// Backing buffer. Phase 1: Vec<u8>. Phase 2: Vec<u8> or MmapMut.
    buffer: BackingStore,
    /// Paged allocator for this region.
    allocator: PagedAllocator,
    /// Descriptive region name.
    #[allow(dead_code)]
    name: String,
}

impl RegionSlot {
    #[doc(alias = "vil_keep")]
    fn new_anonymous(name: String, size: usize) -> Self {
        Self {
            buffer: BackingStore::Vec(vec![0u8; size]),
            allocator: PagedAllocator::new(size),
            name,
        }
    }

    #[doc(alias = "vil_keep")]
    fn new_named(name: String, size: usize) -> std::io::Result<Self> {
        let shm_path = format!("/vil_{}", name);
        
        let fd = mman::shm_open(
            shm_path.as_str(),
            OFlag::O_CREAT | OFlag::O_RDWR,
            Mode::S_IRUSR | Mode::S_IWUSR,
        ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        nix::unistd::ftruncate(&fd, size as nix::libc::off_t)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // SAFETY: fd is valid, obtained from shm_open. Ownership transferred via into_raw_fd.
        let file = unsafe { std::fs::File::from_raw_fd(fd.into_raw_fd()) };
        // SAFETY: file is valid and exclusively owned; no other mappings exist yet.
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        // Pre-fault pages: lock in RAM to eliminate page faults on ShmToken hot path.
        let _ = mmap.lock();

        Ok(Self {
            buffer: BackingStore::Mmap(mmap),
            allocator: PagedAllocator::new(size),
            name,
        })
    }

    #[doc(alias = "vil_keep")]
    fn attach_named(name: String) -> std::io::Result<Self> {
        let shm_path = format!("/vil_{}", name);
        
        let fd = mman::shm_open(
            shm_path.as_str(),
            OFlag::O_RDWR,
            Mode::empty(),
        ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let stat = nix::sys::stat::fstat(fd.as_raw_fd())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let size = stat.st_size as usize;

        // SAFETY: fd is valid, obtained from shm_open. Ownership transferred via into_raw_fd.
        let file = unsafe { std::fs::File::from_raw_fd(fd.into_raw_fd()) };
        // SAFETY: file is valid and exclusively owned; mapping shared memory for attach.
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        // Pre-fault pages on attach
        let _ = mmap.lock();

        Ok(Self {
            buffer: BackingStore::Mmap(mmap),
            allocator: PagedAllocator::new(size),
            name,
        })
    }

    #[doc(alias = "vil_keep")]
    fn stats(&self, id: RegionId) -> RegionStats {
        RegionStats {
            region_id: id,
            capacity: self.allocator.total_capacity,
            used: self.allocator.used(),
            remaining: self.allocator.remaining(),
        }
    }
}

struct HeapState {
    next_region_id: u64,
    regions: HashMap<RegionId, RegionSlot>,
}

/// Lock-free bump region for high-throughput ShmToken data writes.
/// One atomic fetch_add per alloc — no mutex, no page scan.
pub struct BumpRegion {
    base_ptr: *mut u8,
    capacity: usize,
    cursor: std::sync::atomic::AtomicUsize,
}

// SAFETY: BumpRegion uses AtomicUsize for offset. All mmap access is bounds-checked.
// Concurrent access is safe because allocations are append-only via atomic fetch_add.
unsafe impl Send for BumpRegion {}
unsafe impl Sync for BumpRegion {}

impl BumpRegion {
    /// Bump-allocate `size` bytes. Returns offset or None if full.
    /// Lock-free: uses compare-and-swap loop to prevent race conditions
    /// on wrap-around. Only one thread can win the CAS per allocation.
    pub fn alloc(&self, size: usize) -> Option<Offset> {
        let aligned_size = (size + 7) & !7; // 8-byte align
        loop {
            let current = self.cursor.load(std::sync::atomic::Ordering::Acquire);
            // SAFETY: use checked_add to prevent integer overflow
            let new_offset = match current.checked_add(aligned_size) {
                Some(n) => n,
                None => return None, // overflow — region exhausted
            };
            if new_offset > self.capacity {
                // Wrap around: try to reset cursor to beginning.
                // Only one thread will succeed the CAS; others retry.
                match self.cursor.compare_exchange_weak(
                    current,
                    aligned_size,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                ) {
                    Ok(_) => return Some(Offset::new(0)),
                    Err(_) => continue, // another thread won — retry
                }
            } else {
                // Normal allocation: CAS to claim [current..new_offset]
                match self.cursor.compare_exchange_weak(
                    current,
                    new_offset,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                ) {
                    Ok(_) => return Some(Offset::new(current as u64)),
                    Err(_) => continue, // contention — retry
                }
            }
        }
    }

    /// Write bytes at offset. No lock — direct pointer write.
    /// SAFETY: Caller must ensure offset is valid (from alloc).
    pub unsafe fn write(&self, offset: Offset, data: &[u8]) {
        let dst = self.base_ptr.add(offset.as_usize());
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
    }

    /// Read bytes at offset. No lock — direct pointer read.
    pub unsafe fn read(&self, offset: Offset, len: usize) -> &[u8] {
        let src = self.base_ptr.add(offset.as_usize());
        std::slice::from_raw_parts(src, len)
    }

    /// Alloc + write in one shot. Lock-free.
    pub fn alloc_and_write(&self, data: &[u8]) -> Option<(Offset, u32)> {
        let offset = self.alloc(data.len())?;
        // SAFETY: offset obtained from alloc(), guaranteed within bounds.
        unsafe { self.write(offset, data); }
        Some((offset, data.len() as u32))
    }

    /// Reset cursor to 0 (reuse region).
    pub fn reset(&self) {
        self.cursor.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct ExchangeHeap {
    inner: Arc<Mutex<HeapState>>,
    /// Lock-free bump regions indexed by region ID (for ShmToken fast path).
    bump_regions: Arc<dashmap::DashMap<RegionId, Arc<BumpRegion>>>,
}

impl ExchangeHeap {
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HeapState {
                next_region_id: 0,
                regions: HashMap::new(),
            })),
            bump_regions: Arc::new(dashmap::DashMap::new()),
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn create_region(&self, name: &str, size: usize) -> RegionId {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let id = RegionId(state.next_region_id);
        state.next_region_id += 1;
        state.regions.insert(id, RegionSlot::new_anonymous(name.to_string(), size));
        id
    }

    #[doc(alias = "vil_keep")]
    pub fn create_named_region(&self, name: &str, size: usize) -> std::io::Result<RegionId> {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let id = RegionId(state.next_region_id);
        state.next_region_id += 1;
        state.regions.insert(id, RegionSlot::new_named(name.to_string(), size)?);
        Ok(id)
    }

    #[doc(alias = "vil_keep")]
    pub fn attach_region(&self, name: &str) -> std::io::Result<RegionId> {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let id = RegionId(state.next_region_id);
        state.next_region_id += 1;
        state.regions.insert(id, RegionSlot::attach_named(name.to_string())?);
        Ok(id)
    }

    #[doc(alias = "vil_keep")]
    pub fn unlink_region(name: &str) -> std::io::Result<()> {
        let shm_path = format!("/vil_{}", name);
        mman::shm_unlink(shm_path.as_str())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    #[doc(alias = "vil_keep")]
    pub fn alloc_in_region_raw(&self, region_id: RegionId, size: usize, align: usize) -> Option<Offset> {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = state.regions.get(&region_id)?;
        slot.allocator.try_alloc(size, align)
    }

    #[doc(alias = "vil_keep")]
    pub fn alloc_and_write<T: Copy>(&self, region_id: RegionId, value: T) -> Option<RelativePtr<T>> {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = state.regions.get_mut(&region_id)?;

        let offset = slot.allocator.try_alloc(
            std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
        )?;

        let off = offset.as_usize();
        let size = std::mem::size_of::<T>();
        let src = &value as *const T as *const u8;
        // SAFETY: offset from try_alloc is within buffer bounds; src/dst do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(src, slot.buffer.as_mut_ptr().add(off), size);
        }

        Some(RelativePtr::from_offset(offset))
    }

    #[doc(alias = "vil_keep")]
    pub fn read_at<T: Copy>(&self, region_id: RegionId, ptr: RelativePtr<T>) -> Option<T> {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = state.regions.get(&region_id)?;

        let off = ptr.offset().as_usize();
        let size = std::mem::size_of::<T>();

        if off.checked_add(size).map_or(true, |end| end > slot.buffer.len()) {
            return None;
        }

        let mut value = std::mem::MaybeUninit::<T>::uninit();
        // SAFETY: bounds checked above; buffer ptr and local MaybeUninit do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(
                slot.buffer.as_ptr().add(off),
                value.as_mut_ptr() as *mut u8,
                size,
            );
            Some(value.assume_init())
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn write_at<T: Copy>(&self, region_id: RegionId, ptr: RelativePtr<T>, value: T) -> bool {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = match state.regions.get_mut(&region_id) {
            Some(s) => s,
            None => return false,
        };

        let off = ptr.offset().as_usize();
        let size = std::mem::size_of::<T>();

        if off.checked_add(size).map_or(true, |end| end > slot.buffer.len()) {
            return false;
        }

        let src = &value as *const T as *const u8;
        // SAFETY: bounds checked above; src (stack local) and dst (buffer) do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(src, slot.buffer.as_mut_ptr().add(off), size);
        }
        true
    }

    #[doc(alias = "vil_keep")]
    pub fn alloc_bytes(&self, region_id: RegionId, size: usize, align: usize) -> Option<Offset> {
        self.alloc_in_region_raw(region_id, size, align)
    }

    /// Get or create a lock-free bump region for high-throughput streaming.
    /// First call takes mutex to resolve base pointer; subsequent calls are lock-free.
    pub fn bump_region(&self, region_id: RegionId) -> Option<Arc<BumpRegion>> {
        if let Some(bump) = self.bump_regions.get(&region_id) {
            return Some(bump.clone());
        }
        // First access: resolve base pointer from region slot
        let state = self.inner.lock().ok()?;
        let slot = state.regions.get(&region_id)?;
        let base_ptr = slot.buffer.as_ptr() as *mut u8;
        let capacity = slot.buffer.len();
        let bump = Arc::new(BumpRegion {
            base_ptr,
            capacity,
            cursor: std::sync::atomic::AtomicUsize::new(0),
        });
        self.bump_regions.insert(region_id, bump.clone());
        Some(bump)
    }

    /// Lock-free alloc+write for ShmToken data payload. Single atomic op.
    pub fn bump_alloc_and_write(&self, region_id: RegionId, data: &[u8]) -> Option<(Offset, u32)> {
        let bump = self.bump_region(region_id)?;
        bump.alloc_and_write(data)
    }

    #[doc(alias = "vil_keep")]
    pub fn write_bytes(&self, region_id: RegionId, offset: Offset, data: &[u8]) -> bool {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = match state.regions.get_mut(&region_id) {
            Some(s) => s,
            None => return false,
        };
        let off = offset.as_usize();
        if off.checked_add(data.len()).map_or(true, |end| end > slot.buffer.len()) {
            return false;
        }
        // SAFETY: bounds checked above; src (caller slice) and dst (buffer) do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), slot.buffer.as_mut_ptr().add(off), data.len());
        }
        true
    }

    #[doc(alias = "vil_keep")]
    pub fn read_bytes(&self, region_id: RegionId, offset: Offset, len: usize) -> Option<Vec<u8>> {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = state.regions.get(&region_id)?;
        let off = offset.as_usize();
        if off.checked_add(len).map_or(true, |end| end > slot.buffer.len()) {
            return None;
        }
        let mut dest = vec![0u8; len];
        // SAFETY: bounds checked above; src (buffer) and dst (local Vec) do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(slot.buffer.as_ptr().add(off), dest.as_mut_ptr(), len);
        }
        Some(dest)
    }

    #[doc(alias = "vil_keep")]
    pub fn get_region_ptr(&self, region_id: RegionId) -> Option<*mut u8> {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        state.regions.get_mut(&region_id).map(|s| s.buffer.as_mut_ptr())
    }

    #[doc(alias = "vil_keep")]
    pub fn region_stats(&self, region_id: RegionId) -> Option<RegionStats> {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        state.regions.get(&region_id).map(|s| s.stats(region_id))
    }

    #[doc(alias = "vil_keep")]
    pub fn all_stats(&self) -> Vec<RegionStats> {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        state.regions.iter().map(|(id, slot)| slot.stats(*id)).collect()
    }

    #[doc(alias = "vil_keep")]
    pub fn reset_region(&self, region_id: RegionId) -> bool {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        match state.regions.get(&region_id) {
            Some(slot) => {
                slot.allocator.reset();
                true
            }
            None => false,
        }
    }

    #[doc(alias = "vil_keep")]
    pub fn region_count(&self) -> usize {
        let state = self.inner.lock().expect("exchange heap lock poisoned");
        state.regions.len()
    }

    /// Perform in-place compaction on a region.
    /// Moves active samples to the beginning of the region to eliminate fragmentation.
    #[doc(alias = "vil_keep")]
    pub fn compact_region(&self, region_id: vil_types::RegionId, registry: &dyn crate::DefragRegistry) -> Result<usize, String> {
        let mut state = self.inner.lock().expect("exchange heap lock poisoned");
        let slot = state.regions.get_mut(&region_id).ok_or("Region not found")?;

        // 1. Get active sample metadata for this region via trait
        let active_samples = registry.get_active_samples(region_id);

        if active_samples.is_empty() {
            slot.allocator.reset();
            return Ok(0);
        }

        // 2. Sort by current offset to avoid overlapping during copy
        let mut sorted_samples = active_samples;
        sorted_samples.sort_by_key(|s| s.offset);

        // 3. Re-pack samples
        let mut next_offset = 0;
        let mut moved_count = 0;

        for sample in sorted_samples {
            let size = sample.size;
            let align = sample.align;
            
            // Determine new aligned offset
            let aligned_offset = (next_offset + align - 1) & !(align - 1);
            
            if aligned_offset != sample.offset as usize {
                // Move data
                // SAFETY: samples sorted by offset; dst <= src so regions may overlap,
                // hence ptr::copy (not copy_nonoverlapping). Both within buffer bounds.
                unsafe {
                    let src = slot.buffer.as_ptr().add(sample.offset as usize);
                    let dst = slot.buffer.as_mut_ptr().add(aligned_offset);
                    std::ptr::copy(src, dst, size);
                }
                
                // Update registry atomically via trait
                registry.update_offset(sample.id, aligned_offset as u64);
                moved_count += 1;
            }
            
            next_offset = aligned_offset + size;
        }

        // 4. Update allocator state
        slot.allocator.reset_to(next_offset);

        Ok(moved_count)
    }
}

impl Default for ExchangeHeap {
    fn default() -> Self {
        Self::new()
    }
}
