// =============================================================================
// vil_shm::offset — Relative Offset Addressing
// =============================================================================
// Core of VIL zero-copy: all references crossing boundaries must be
// relative offsets, NOT absolute pointers.
//
// Formula: local_addr = base_ptr + offset
//
// RelativePtr<T> stores an offset (not a pointer), making it safe for:
//   - shared memory across processes
//   - relocatable regions
//   - serialization without rewriting
//
// TASK LIST:
// [x] Offset — raw offset value type
// [x] RelativePtr<T> — typed relative pointer (offset-based)
// [x] resolve / resolve_mut — convert offset to local pointer
// [x] Unit tests
// =============================================================================

use std::marker::PhantomData;

/// Raw byte offset relative to the region base.
///
/// Offset(0) points to the start of the region.
/// Offset is Copy and cheap — just a u64.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Offset(pub u64);

impl Offset {
    pub const ZERO: Offset = Offset(0);

    /// Create a new offset.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Offset value in bytes.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Offset value as usize (for indexing).
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Offset({})", self.0)
    }
}

/// Typed relative pointer — stores an offset, not an absolute pointer.
///
/// `RelativePtr<T>` is safe for shared memory because it contains no
/// absolute address. To access data, it must be resolved against
/// the region's base pointer.
///
/// # Safety
/// - Caller must ensure the offset is valid within the region
/// - Caller must ensure T matches the data at that offset
/// - Caller must ensure alignment of T is satisfied
#[derive(Debug)]
pub struct RelativePtr<T> {
    offset: Offset,
    _marker: PhantomData<T>,
}

// Manual Clone/Copy because PhantomData<T> does not affect layout
impl<T> Clone for RelativePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for RelativePtr<T> {}

impl<T> PartialEq for RelativePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

impl<T> Eq for RelativePtr<T> {}

impl<T> RelativePtr<T> {
    /// Create a RelativePtr from an offset.
    pub fn from_offset(offset: Offset) -> Self {
        Self {
            offset,
            _marker: PhantomData,
        }
    }

    /// Get the raw offset.
    pub fn offset(self) -> Offset {
        self.offset
    }

    /// Resolve RelativePtr to a reference using a base pointer.
    ///
    /// # Safety
    /// - `base` must be a valid pointer to the region start
    /// - offset + size_of::<T>() must not exceed the region
    /// - data at offset must be initialized as T
    /// - alignment of T must be satisfied at base + offset
    pub unsafe fn resolve(self, base: *const u8) -> &'static T {
        let ptr = base.add(self.offset.as_usize()) as *const T;
        &*ptr
    }

    /// Resolve RelativePtr to a mutable reference.
    ///
    /// # Safety
    /// Same as `resolve`, plus:
    /// - no other mutable reference to this data may exist
    pub unsafe fn resolve_mut(self, base: *mut u8) -> &'static mut T {
        let ptr = base.add(self.offset.as_usize()) as *mut T;
        &mut *ptr
    }
}

impl<T> std::fmt::Display for RelativePtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RelativePtr<{}>({})",
            std::any::type_name::<T>(),
            self.offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_creation() {
        let o = Offset::new(128);
        assert_eq!(o.as_u64(), 128);
        assert_eq!(o.as_usize(), 128);
    }

    #[test]
    fn test_offset_zero() {
        assert_eq!(Offset::ZERO.as_u64(), 0);
    }

    #[test]
    fn test_relative_ptr_copy_eq() {
        let p1 = RelativePtr::<u64>::from_offset(Offset(42));
        let p2 = p1; // Copy
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_resolve_basic() {
        // Simulate a small region
        let mut buffer = vec![0u8; 64];

        // Write u32 value 0xDEADBEEF at offset 8
        let value: u32 = 0xDEAD_BEEF;
        let offset = 8usize;
        unsafe {
            let ptr = buffer.as_mut_ptr().add(offset) as *mut u32;
            ptr.write(value);
        }

        // Resolve via RelativePtr
        let rptr = RelativePtr::<u32>::from_offset(Offset(offset as u64));
        let resolved = unsafe { rptr.resolve(buffer.as_ptr()) };
        assert_eq!(*resolved, 0xDEAD_BEEF);
    }

    #[test]
    fn test_resolve_mut() {
        let mut buffer = vec![0u8; 64];
        let offset = 16usize;

        let rptr = RelativePtr::<u64>::from_offset(Offset(offset as u64));

        // Write via resolve_mut
        unsafe {
            let val = rptr.resolve_mut(buffer.as_mut_ptr());
            *val = 12345;
        }

        // Read via resolve
        let val = unsafe { rptr.resolve(buffer.as_ptr()) };
        assert_eq!(*val, 12345);
    }
}
