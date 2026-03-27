// =============================================================================
// VilModel — Zero-Copy Model Trait for VIL Server Handlers
// =============================================================================
//
// Types implementing VilModel can be deserialized from ShmSlice (zero-copy)
// and serialized to Bytes (for SHM write-through or HTTP response).
//
// Use `#[derive(VilModel)]` from `vil_macros` for automatic implementation.
// The struct must also derive `Serialize`, `Deserialize`, and `Clone`.

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for VIL data models with SHM-aware serialization.
///
/// Implement this via `#[derive(VilModel)]` from `vil_macros` — manual
/// implementation is possible but not recommended.
///
/// # Requirements
///
/// The implementing type must also derive:
/// - `serde::Serialize`
/// - `serde::Deserialize`
/// - `Clone`
///
/// # Example
///
/// ```ignore
/// use serde::{Serialize, Deserialize};
/// use vil_macros::VilModel;
///
/// #[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// struct Task {
///     id: u64,
///     title: String,
///     done: bool,
/// }
/// ```
pub trait VilModel: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Deserialize from an ShmSlice (or raw bytes) using vil_json.
    fn from_shm_bytes(bytes: &[u8]) -> Result<Self, crate::VilError>;

    /// Serialize to Bytes using vil_json.
    fn to_json_bytes(&self) -> Result<Bytes, crate::VilError>;
}
