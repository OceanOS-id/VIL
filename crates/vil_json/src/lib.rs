// =============================================================================
// vil_json — VIL Zero-Copy JSON Abstraction
// =============================================================================
//
// Provides a unified JSON API with optional SIMD acceleration.
//
// Default backend: serde_json (always available, zero extra deps).
// Feature "simd": sonic-rs backend (~2-3x faster for payloads >256B).
//
// All VIL server hot paths should use vil_json instead of serde_json
// directly, so that enabling the "simd" feature automatically accelerates
// the entire stack.
//
// Design principle: same API surface regardless of backend. The caller
// never needs to know which engine is active.

use serde::{de::DeserializeOwned, Serialize};

// =============================================================================
// Error Type
// =============================================================================

/// Unified JSON error wrapping either serde_json or sonic-rs errors.
#[derive(Debug)]
pub struct JsonError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON error: {}", self.inner)
    }
}

impl std::error::Error for JsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        Self { inner: Box::new(e) }
    }
}

#[cfg(feature = "simd")]
impl From<sonic_rs::Error> for JsonError {
    fn from(e: sonic_rs::Error) -> Self {
        Self { inner: Box::new(e) }
    }
}

// =============================================================================
// Deserialization
// =============================================================================

/// Deserialize a value from a byte slice.
///
/// With the "simd" feature enabled, uses sonic-rs for SIMD-accelerated parsing.
/// sonic-rs accepts `&[u8]` (immutable), making it compatible with ShmSlice's
/// `Bytes` backing store without extra copies.
///
/// Without "simd", delegates to serde_json::from_slice.
#[inline]
pub fn from_slice<T: DeserializeOwned>(data: &[u8]) -> Result<T, JsonError> {
    #[cfg(feature = "simd")]
    {
        sonic_rs::from_slice(data).map_err(JsonError::from)
    }
    #[cfg(not(feature = "simd"))]
    {
        serde_json::from_slice(data).map_err(JsonError::from)
    }
}

/// Deserialize a value from a string.
#[inline]
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, JsonError> {
    #[cfg(feature = "simd")]
    {
        sonic_rs::from_str(s).map_err(JsonError::from)
    }
    #[cfg(not(feature = "simd"))]
    {
        serde_json::from_str(s).map_err(JsonError::from)
    }
}

// =============================================================================
// Serialization
// =============================================================================

/// Serialize a value to a Vec<u8>.
#[inline]
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    #[cfg(feature = "simd")]
    {
        sonic_rs::to_vec(value).map_err(JsonError::from)
    }
    #[cfg(not(feature = "simd"))]
    {
        serde_json::to_vec(value).map_err(JsonError::from)
    }
}

/// Serialize a value to a String.
#[inline]
pub fn to_string<T: Serialize>(value: &T) -> Result<String, JsonError> {
    #[cfg(feature = "simd")]
    {
        sonic_rs::to_string(value).map_err(JsonError::from)
    }
    #[cfg(not(feature = "simd"))]
    {
        serde_json::to_string(value).map_err(JsonError::from)
    }
}

/// Serialize a value to `bytes::Bytes` (zero-copy compatible).
///
/// Returns owned Bytes suitable for SHM write-through or HTTP response body.
#[inline]
pub fn to_bytes<T: Serialize>(value: &T) -> Result<bytes::Bytes, JsonError> {
    let vec = to_vec(value)?;
    Ok(bytes::Bytes::from(vec))
}

// =============================================================================
// Re-exports for convenience
// =============================================================================

/// Re-export serde_json::Value for ad-hoc JSON construction.
/// Prefer typed structs with #[derive(VilModel)] when possible.
pub use serde_json::Value;

// =============================================================================
// vil_json!{} macro — typed JSON literal construction
// =============================================================================

/// JSON literal construction returning `VilJsonValue`.
///
/// This wraps `serde_json::json!` but returns a `VilJsonValue` that
/// provides `.to_bytes()` for SHM-compatible serialization.
///
/// # Example
/// ```
/// use vil_json::vil_json;
///
/// let val = vil_json!({
///     "id": 42,
///     "name": "test"
/// });
/// let bytes = val.to_bytes().unwrap();
/// ```
#[macro_export]
macro_rules! vil_json {
    ($($json:tt)+) => {
        $crate::VilJsonValue($crate::__private::json_internal!($($json)+))
    };
}

/// Wrapper over `serde_json::Value` with VIL integration methods.
#[derive(Debug, Clone)]
pub struct VilJsonValue(pub serde_json::Value);

impl VilJsonValue {
    /// Serialize to `bytes::Bytes` for SHM or HTTP response.
    #[inline]
    pub fn to_bytes(&self) -> Result<bytes::Bytes, JsonError> {
        to_bytes(&self.0)
    }

    /// Serialize to `Vec<u8>`.
    #[inline]
    pub fn to_vec(&self) -> Result<Vec<u8>, JsonError> {
        to_vec(&self.0)
    }

    /// Serialize to `String`.
    #[inline]
    pub fn to_json_string(&self) -> Result<String, JsonError> {
        to_string(&self.0)
    }

    /// Access the inner serde_json::Value.
    #[inline]
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    /// Consume and return the inner serde_json::Value.
    #[inline]
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
}

impl Serialize for VilJsonValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl std::fmt::Display for VilJsonValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Private module for macro hygiene
#[doc(hidden)]
pub mod __private {
    pub use serde_json::json as json_internal;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        id: u64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_roundtrip() {
        let original = TestPayload {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = to_vec(&original).unwrap();
        let decoded: TestPayload = from_slice(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_to_string() {
        let payload = TestPayload {
            id: 1,
            name: "hello".to_string(),
            active: false,
        };

        let s = to_string(&payload).unwrap();
        assert!(s.contains("\"id\":1"));
        assert!(s.contains("\"name\":\"hello\""));
    }

    #[test]
    fn test_to_bytes() {
        let payload = TestPayload {
            id: 99,
            name: "bytes".to_string(),
            active: true,
        };

        let b = to_bytes(&payload).unwrap();
        assert!(!b.is_empty());

        let decoded: TestPayload = from_slice(&b).unwrap();
        assert_eq!(payload, decoded);
    }

    #[test]
    fn test_from_str() {
        let json = r#"{"id":10,"name":"fromstr","active":true}"#;
        let decoded: TestPayload = from_str(json).unwrap();
        assert_eq!(decoded.id, 10);
        assert_eq!(decoded.name, "fromstr");
    }

    #[test]
    fn test_vil_json_macro() {
        let val = vil_json!({
            "id": 42,
            "name": "macro_test"
        });

        assert_eq!(val.as_value()["id"], 42);
        assert_eq!(val.as_value()["name"], "macro_test");

        let bytes = val.to_bytes().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_error_display() {
        let result: Result<TestPayload, JsonError> = from_slice(b"not json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("JSON error"));
    }
}
