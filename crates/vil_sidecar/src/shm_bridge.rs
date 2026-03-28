// =============================================================================
// Sidecar SHM Bridge — Shared memory region for host ↔ sidecar data exchange
// =============================================================================
//
// Each sidecar gets its own SHM region: /dev/shm/vil_sc_{name}
// Both host and sidecar mmap the same file for zero-copy data transfer.
//
// Layout:
//   [Header: 64 bytes] [Data area: remaining bytes]
//
// The header contains a write cursor (atomic u64) so both sides know
// where free space starts. Simple bump allocator — reset when full.

use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::sync::atomic::{AtomicU64, Ordering};

/// Default SHM region size: 64 MB.
pub const DEFAULT_SHM_SIZE: u64 = 64 * 1024 * 1024;

/// Header size at the start of the SHM region.
const HEADER_SIZE: u64 = 64;

/// SHM bridge error.
#[derive(Debug)]
pub enum ShmBridgeError {
    Io(std::io::Error),
    RegionFull { requested: u32, available: u64 },
    InvalidOffset { offset: u64, len: u32, region_size: u64 },
}

impl std::fmt::Display for ShmBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "SHM I/O error: {}", e),
            Self::RegionFull { requested, available } => {
                write!(f, "SHM region full: requested {} bytes, {} available", requested, available)
            }
            Self::InvalidOffset { offset, len, region_size } => {
                write!(f, "invalid SHM offset: offset={}, len={}, region_size={}", offset, len, region_size)
            }
        }
    }
}

impl std::error::Error for ShmBridgeError {}

impl From<std::io::Error> for ShmBridgeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

// ---------------------------------------------------------------------------
// ShmRegion — memory-mapped shared region
// ---------------------------------------------------------------------------

/// A shared memory region for sidecar data exchange.
///
/// Both host and sidecar open the same file and mmap it.
/// Data is written via `write()` which returns an (offset, len) pair.
/// Data is read via `read()` given an (offset, len) pair.
pub struct ShmRegion {
    mmap: MmapMut,
    size: u64,
    path: String,
}

impl ShmRegion {
    /// Create a new SHM region at the given path with the given size.
    /// If the file already exists, it is truncated and reused.
    pub fn create(path: impl Into<String>, size: u64) -> Result<Self, ShmBridgeError> {
        let path = path.into();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.set_len(size)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        let region = Self { mmap, size, path };
        // Initialize header: write cursor at HEADER_SIZE (start of data area)
        region.write_cursor().store(HEADER_SIZE, Ordering::Release);
        {
            use vil_log::{system_log, types::SystemPayload};
            system_log!(Info, SystemPayload {
                event_type: 4,
                mem_kb: (size / 1024) as u32,
                ..Default::default()
            });
        }

        Ok(region)
    }

    /// Open an existing SHM region (for the sidecar side).
    pub fn open(path: impl Into<String>) -> Result<Self, ShmBridgeError> {
        let path = path.into();
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        {
            use vil_log::{system_log, types::SystemPayload};
            system_log!(Info, SystemPayload {
                event_type: 4,
                mem_kb: (size / 1024) as u32,
                ..Default::default()
            });
        }
        Ok(Self { mmap, size, path })
    }

    /// Write data to the region. Returns (offset, len) for the descriptor.
    ///
    /// Uses a bump allocator: atomically advances the write cursor.
    /// When the region is full, call `reset()` to reclaim space.
    pub fn write(&self, data: &[u8]) -> Result<(u64, u32), ShmBridgeError> {
        let len = data.len() as u32;
        let aligned_len = align_up(len as u64, 8); // 8-byte alignment

        // Atomic bump allocation
        let cursor = self.write_cursor();
        let offset = cursor.fetch_add(aligned_len, Ordering::AcqRel);

        // Check bounds
        if offset + aligned_len > self.size {
            // Roll back the cursor (best-effort)
            cursor.fetch_sub(aligned_len, Ordering::AcqRel);
            return Err(ShmBridgeError::RegionFull {
                requested: len,
                available: self.size.saturating_sub(offset),
            });
        }

        // Write data via raw pointer (safe: we own the allocation via atomic cursor)
        unsafe {
            let dst = self.mmap.as_ptr().add(offset as usize) as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }

        Ok((offset, len))
    }

    /// Read data from the region at the given offset and length.
    pub fn read(&self, offset: u64, len: u32) -> Result<&[u8], ShmBridgeError> {
        let end = offset + len as u64;
        if end > self.size {
            return Err(ShmBridgeError::InvalidOffset {
                offset,
                len,
                region_size: self.size,
            });
        }
        Ok(&self.mmap[offset as usize..end as usize])
    }

    /// Reset the write cursor to the start of the data area.
    /// Only safe when no readers are active on existing data.
    pub fn reset(&self) {
        self.write_cursor().store(HEADER_SIZE, Ordering::Release);
        {
            use vil_log::app_log;
            app_log!(Debug, "sidecar.shm.reset", { path: vil_log::dict::register_str(&self.path) as u64 });
        }
    }

    /// Current write cursor position (bytes used including header).
    pub fn cursor_position(&self) -> u64 {
        self.write_cursor().load(Ordering::Acquire)
    }

    /// Available bytes in the data area.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.cursor_position())
    }

    /// Total size of the region.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Path of the SHM file.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Utilization as a percentage (0.0 - 100.0).
    pub fn utilization_pct(&self) -> f64 {
        let used = self.cursor_position() - HEADER_SIZE;
        let total = self.size - HEADER_SIZE;
        if total == 0 {
            return 0.0;
        }
        (used as f64 / total as f64) * 100.0
    }

    // Internal: get the write cursor as an atomic reference.
    fn write_cursor(&self) -> &AtomicU64 {
        // Safety: header is at offset 0, aligned to 8 bytes, and we own the mmap.
        unsafe { &*(self.mmap.as_ptr() as *const AtomicU64) }
    }
}

impl Drop for ShmRegion {
    fn drop(&mut self) {
        // Don't remove the file here — the other side may still be using it.
        // Cleanup is handled by the lifecycle manager.
        {
            use vil_log::app_log;
            app_log!(Debug, "sidecar.shm.dropped", { path: vil_log::dict::register_str(&self.path) as u64 });
        }
    }
}

/// Remove a SHM region file. Call this during cleanup.
pub fn remove_shm_region(path: &str) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            {
                use vil_log::app_log;
                app_log!(Warn, "sidecar.shm.remove.failed", { path: path, error: e.to_string() });
            }
        }
    }
}

/// Align a value up to the given alignment.
fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_write_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_shm");
        let path_str = path.to_str().unwrap();

        let region = ShmRegion::create(path_str, 4096).unwrap();

        let data = b"hello sidecar world";
        let (offset, len) = region.write(data).unwrap();

        assert_eq!(offset, HEADER_SIZE); // First write starts after header
        assert_eq!(len, data.len() as u32);

        let read_back = region.read(offset, len).unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_multiple_writes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("multi_shm");
        let path_str = path.to_str().unwrap();

        let region = ShmRegion::create(path_str, 4096).unwrap();

        let (off1, len1) = region.write(b"first").unwrap();
        let (off2, len2) = region.write(b"second").unwrap();

        // Second write should be 8-byte aligned after first
        assert!(off2 > off1);
        assert_eq!(off2, HEADER_SIZE + 8); // "first" = 5 bytes, aligned to 8

        assert_eq!(region.read(off1, len1).unwrap(), b"first");
        assert_eq!(region.read(off2, len2).unwrap(), b"second");
    }

    #[test]
    fn test_region_full() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("full_shm");
        let path_str = path.to_str().unwrap();

        // Tiny region: 128 bytes (64 header + 64 data)
        let region = ShmRegion::create(path_str, 128).unwrap();

        // Write 56 bytes — fits (64 data area, 56 < 64)
        let data = vec![0u8; 56];
        region.write(&data).unwrap();

        // Write 16 more — doesn't fit
        let result = region.write(&vec![0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reset_shm");
        let path_str = path.to_str().unwrap();

        let region = ShmRegion::create(path_str, 4096).unwrap();

        region.write(b"data1").unwrap();
        region.write(b"data2").unwrap();

        let used = region.cursor_position();
        assert!(used > HEADER_SIZE);

        region.reset();
        assert_eq!(region.cursor_position(), HEADER_SIZE);

        // Can write again
        region.write(b"fresh").unwrap();
    }

    #[test]
    fn test_open_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("open_shm");
        let path_str = path.to_str().unwrap();

        // Create and write
        let region = ShmRegion::create(path_str, 4096).unwrap();
        let (offset, len) = region.write(b"shared data").unwrap();
        drop(region);

        // Open and read
        let region2 = ShmRegion::open(path_str).unwrap();
        let data = region2.read(offset, len).unwrap();
        assert_eq!(data, b"shared data");
    }

    #[test]
    fn test_utilization() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("util_shm");
        let path_str = path.to_str().unwrap();

        let region = ShmRegion::create(path_str, 1024).unwrap();
        assert!(region.utilization_pct() < 0.01);

        // Write half of data area (1024 - 64 = 960 data bytes)
        region.write(&vec![0u8; 480]).unwrap();
        assert!(region.utilization_pct() > 49.0);
        assert!(region.utilization_pct() < 51.0);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
    }
}
