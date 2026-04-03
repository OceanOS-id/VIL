// =============================================================================
// VIL Upload — File upload extractor with validation
// =============================================================================

use axum::extract::FromRequest;
use axum::http::Request;
use bytes::Bytes;
use serde::Serialize;
use std::path::Path;

use crate::state::AppState;
use crate::VilError;

/// Uploaded file with content type detection and validation.
///
/// # Example
/// ```ignore
/// #[vil_handler]
/// async fn upload(file: VilUpload) -> VilResult<SavedFile> {
///     let saved = file
///         .validate_type(&["image/png", "image/jpeg"])?
///         .max_size(10 * 1024 * 1024)?
///         .save_to("uploads/avatars")?;
///     Ok(VilResponse::created(saved))
/// }
/// ```
pub struct VilUpload {
    /// Raw file bytes
    pub bytes: Bytes,
    /// Detected content type (from magic bytes, not header)
    pub content_type: String,
    /// File size in bytes
    pub size: usize,
}

/// Result of saving an uploaded file.
#[derive(Debug, Clone, Serialize)]
pub struct SavedFile {
    /// Filesystem path
    pub path: String,
    /// Public URL path (e.g. "/uploads/avatars/abc123.png")
    pub url: String,
    /// File size in bytes
    pub size: usize,
    /// Content type
    pub content_type: String,
}

impl VilUpload {
    /// Validate content type via magic bytes.
    /// Returns error if content type not in allowed list.
    pub fn validate_type(self, allowed: &[&str]) -> Result<Self, VilError> {
        if !allowed.contains(&self.content_type.as_str()) {
            return Err(VilError::validation(format!(
                "File type '{}' not allowed. Allowed: {:?}",
                self.content_type, allowed
            )));
        }
        Ok(self)
    }

    /// Validate max file size.
    pub fn max_size(self, max_bytes: usize) -> Result<Self, VilError> {
        if self.size > max_bytes {
            return Err(VilError::validation(format!(
                "File too large ({} bytes). Max: {} bytes",
                self.size, max_bytes
            )));
        }
        Ok(self)
    }

    /// Save file to directory with auto-generated UUID filename.
    pub fn save_to(self, dir: &str) -> Result<SavedFile, VilError> {
        let ext = match self.content_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "application/pdf" => "pdf",
            "audio/mpeg" => "mp3",
            "audio/wav" => "wav",
            _ => "bin",
        };
        let filename = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        self.save_as(dir, &filename)
    }

    /// Save file with custom filename.
    pub fn save_as(self, dir: &str, filename: &str) -> Result<SavedFile, VilError> {
        std::fs::create_dir_all(dir)
            .map_err(|e| VilError::internal(format!("create dir: {e}")))?;

        let path = format!("{}/{}", dir, filename);
        std::fs::write(&path, &self.bytes)
            .map_err(|e| VilError::internal(format!("write file: {e}")))?;

        let url = format!("/{}/{}", dir, filename);
        Ok(SavedFile {
            path,
            url,
            size: self.size,
            content_type: self.content_type,
        })
    }
}

/// Detect content type from magic bytes (first few bytes of file).
fn detect_content_type(bytes: &[u8]) -> String {
    if bytes.len() < 4 {
        return "application/octet-stream".into();
    }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png".into()
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg".into()
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif".into()
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp".into()
    } else if bytes.starts_with(b"%PDF") {
        "application/pdf".into()
    } else if bytes.starts_with(&[0xFF, 0xFB]) || bytes.starts_with(b"ID3") {
        "audio/mpeg".into()
    } else if bytes.starts_with(b"RIFF") {
        "audio/wav".into()
    } else {
        "application/octet-stream".into()
    }
}

#[axum::async_trait]
impl FromRequest<AppState> for VilUpload {
    type Rejection = VilError;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| VilError::bad_request(format!("read body: {e}")))?;

        let size = bytes.len();
        let content_type = detect_content_type(&bytes);

        Ok(VilUpload {
            bytes,
            content_type,
            size,
        })
    }
}
