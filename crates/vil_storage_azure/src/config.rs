// =============================================================================
// vil_storage_azure::config — AzureConfig
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for an Azure Blob Storage client.
///
/// # Example
/// ```rust,ignore
/// let cfg = AzureConfig {
///     account: "myaccount".into(),
///     access_key: "base64key==".into(),
///     container: "my-container".into(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    /// Azure storage account name.
    pub account: String,

    /// Azure storage account access key (base64-encoded).
    pub access_key: String,

    /// Name of the blob container to operate on.
    pub container: String,
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            account: String::new(),
            access_key: String::new(),
            container: String::new(),
        }
    }
}
