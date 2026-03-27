// =============================================================================
// vil_storage_s3::stream — Streaming helpers
// =============================================================================
//
// Utilities for collecting a streaming S3 response body into `bytes::Bytes`.
//
// The AWS SDK returns object bodies as `aws_sdk_s3::primitives::ByteStream`.
// `collect_body` drains the stream into a single contiguous `Bytes` buffer.
// For very large objects, callers that need true streaming should work with
// the `ByteStream` directly; this helper is provided for convenience.
// =============================================================================

use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;

use crate::error::S3Fault;
use vil_log::dict::register_str;

/// Collect an S3 `ByteStream` response body into a single `Bytes` buffer.
///
/// Returns `S3Fault::Unknown` if the body cannot be read.
pub async fn collect_body(stream: ByteStream) -> Result<Bytes, S3Fault> {
    stream
        .collect()
        .await
        .map(|aggregated| aggregated.into_bytes())
        .map_err(|e| S3Fault::Unknown {
            message_hash: register_str(&e.to_string()),
        })
}
