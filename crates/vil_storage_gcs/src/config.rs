// =============================================================================
// vil_storage_gcs::config — GcsConfig
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for a Google Cloud Storage client.
///
/// # Example
/// ```rust,ignore
/// let cfg = GcsConfig {
///     bucket: "my-bucket".into(),
///     credentials_path: Some("/path/to/service-account.json".into()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsConfig {
    /// Name of the GCS bucket to operate on.
    pub bucket: String,

    /// Optional path to a service account JSON credentials file.
    /// If `None`, falls back to Application Default Credentials
    /// (GOOGLE_APPLICATION_CREDENTIALS env var, gcloud auth, etc.).
    pub credentials_path: Option<String>,
}

impl Default for GcsConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            credentials_path: None,
        }
    }
}
