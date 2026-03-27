// =============================================================================
// VStr — Relative-Safe String for VIL Zero-Copy Boundary
// =============================================================================
//
// VStr is the VIL equivalent of String for data that crosses process
// boundaries via shared memory. It is backed by VSlice<u8> (which uses
// bytes::Bytes internally), ensuring:
//
//   - No absolute pointers (VASI-compliant)
//   - Zero-copy forwarding across Tri-Lane mesh
//   - UTF-8 validated at construction time
//   - Deref<Target=str> for ergonomic access
//
// Use VStr in VIL message types instead of String when the data may
// cross process boundaries. For handler-local strings, regular String is fine.
//
// Blueprint reference: Section 9.2 "Relative Profile" lists VStr as a
// first-class relative-safe abstraction alongside VRef<T> and VSlice<T>.

use crate::wrappers::VSlice;
use crate::markers::{Vasi, PodLike};
use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;

/// Relative-safe string wrapper for zero-copy SHM boundary crossing.
///
/// Backed by `VSlice<u8>` with UTF-8 invariant enforced at construction.
/// All methods that create a VStr validate UTF-8; after construction,
/// `as_str()` is O(1) and infallible.
#[derive(Clone, Debug)]
pub struct VStr(VSlice<u8>);

impl VStr {
    /// Create a VStr from a string slice. Copies the bytes into Bytes.
    #[inline]
    pub fn new(s: &str) -> Self {
        Self(VSlice::from_bytes(Bytes::copy_from_slice(s.as_bytes())))
    }

    /// Create a VStr from Bytes, validating UTF-8.
    #[inline]
    pub fn from_bytes(b: Bytes) -> Result<Self, std::str::Utf8Error> {
        std::str::from_utf8(&b)?;
        Ok(Self(VSlice::from_bytes(b)))
    }

    /// Create a VStr from a VSlice<u8>, validating UTF-8.
    #[inline]
    pub fn from_vslice(vs: VSlice<u8>) -> Result<Self, std::str::Utf8Error> {
        std::str::from_utf8(vs.as_slice())?;
        Ok(Self(vs))
    }

    /// Get the string slice. O(1), infallible (UTF-8 validated at construction).
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: UTF-8 was validated at construction time.
        unsafe { std::str::from_utf8_unchecked(self.0.as_slice()) }
    }

    /// Get the underlying bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Byte length of the string.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume and return the inner VSlice<u8>.
    #[inline]
    pub fn into_vslice(self) -> VSlice<u8> {
        self.0
    }

    /// Get the inner VSlice<u8> by reference.
    #[inline]
    pub fn as_vslice(&self) -> &VSlice<u8> {
        &self.0
    }
}

// --- Trait Implementations ---

impl Deref for VStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for VStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for VStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl fmt::Display for VStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for VStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for VStr {}

impl PartialEq<str> for VStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for VStr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for VStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl From<&str> for VStr {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for VStr {
    #[inline]
    fn from(s: String) -> Self {
        Self(VSlice::from_bytes(Bytes::from(s.into_bytes())))
    }
}

// VASI-compliant: backed by VSlice<u8> which uses bytes::Bytes (no absolute pointers).
unsafe impl Vasi for VStr {}
unsafe impl PodLike for VStr {}

// Serialize as a regular JSON string.
impl Serialize for VStr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

// Deserialize from a JSON string into VStr (validates UTF-8 implicitly).
impl<'de> Deserialize<'de> for VStr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(VStr::from(s))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_as_str() {
        let vs = VStr::new("hello");
        assert_eq!(vs.as_str(), "hello");
        assert_eq!(vs.len(), 5);
        assert!(!vs.is_empty());
    }

    #[test]
    fn test_empty() {
        let vs = VStr::new("");
        assert!(vs.is_empty());
        assert_eq!(vs.len(), 0);
        assert_eq!(vs.as_str(), "");
    }

    #[test]
    fn test_from_string() {
        let vs = VStr::from("world".to_string());
        assert_eq!(vs.as_str(), "world");
    }

    #[test]
    fn test_from_str() {
        let vs: VStr = "test".into();
        assert_eq!(&*vs, "test");
    }

    #[test]
    fn test_from_bytes_valid_utf8() {
        let b = Bytes::from_static(b"valid utf8");
        let vs = VStr::from_bytes(b).unwrap();
        assert_eq!(vs.as_str(), "valid utf8");
    }

    #[test]
    fn test_from_bytes_invalid_utf8() {
        let b = Bytes::from_static(&[0xFF, 0xFE]);
        assert!(VStr::from_bytes(b).is_err());
    }

    #[test]
    fn test_deref() {
        let vs = VStr::new("deref test");
        let s: &str = &vs;
        assert_eq!(s, "deref test");
        assert!(vs.contains("deref"));
    }

    #[test]
    fn test_display() {
        let vs = VStr::new("display");
        assert_eq!(format!("{}", vs), "display");
    }

    #[test]
    fn test_equality() {
        let a = VStr::new("same");
        let b = VStr::new("same");
        assert_eq!(a, b);
        assert_eq!(a, "same");
        assert_eq!(a, "same".to_string());
    }

    #[test]
    fn test_into_vslice() {
        let vs = VStr::new("vslice");
        let inner = vs.into_vslice();
        assert_eq!(inner.as_slice(), b"vslice");
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = VStr::new("serde test");
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"serde test\"");

        let decoded: VStr = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_unicode() {
        let vs = VStr::new("こんにちは世界 🌍");
        assert_eq!(vs.as_str(), "こんにちは世界 🌍");
        assert!(vs.len() > 0);
    }
}
