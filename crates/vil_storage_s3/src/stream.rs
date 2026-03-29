// =============================================================================
// vil_storage_s3::stream — Streaming helpers (MinIO SDK)
// =============================================================================
//
// MinIO SDK returns GetObjectResponse with ObjectContent that can be consumed
// via to_segmented_bytes(). This module provides a convenience wrapper.
// =============================================================================

use bytes::Bytes;
use minio::s3::builders::ObjectContent;

use crate::error::S3Fault;
use vil_log::dict::register_str;

/// Collect an ObjectContent response into a single Bytes buffer.
pub async fn collect_content(content: ObjectContent) -> Result<Bytes, S3Fault> {
    content
        .to_segmented_bytes()
        .await
        .map(|sb| sb.to_bytes())
        .map_err(|e| S3Fault::Unknown {
            message_hash: register_str(&e.to_string()),
        })
}
