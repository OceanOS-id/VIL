pub mod manifest;
pub mod node_types;
pub mod templates;
pub mod error_catalog;
pub mod errors;

/// SDK path detection — lightweight, no downloads.
pub mod sdk_path {
    use std::path::PathBuf;

    pub fn sdk_dir() -> PathBuf {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("vil-sdk")
    }

    pub fn sdk_current_path() -> PathBuf {
        sdk_dir().join("current")
    }

    pub fn is_sdk_installed() -> bool {
        sdk_current_path().join("internal").exists()
    }
}
