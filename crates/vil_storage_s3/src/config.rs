// =============================================================================
// vil_storage_s3::config — S3Config
// =============================================================================
//
// Configuration for an S3-compatible object storage endpoint.
// Supports AWS S3, MinIO, Cloudflare R2, DigitalOcean Spaces, and any other
// service that implements the AWS S3 API.
//
// Config types may use External layout profile (setup-time data, not hot-path).
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for an S3-compatible object storage client.
///
/// # MinIO example
/// ```rust,ignore
/// let cfg = S3Config {
///     endpoint: Some("http://localhost:9000".into()),
///     region: "us-east-1".into(),
///     access_key: Some("minioadmin".into()),
///     secret_key: Some("minioadmin".into()),
///     bucket: "my-bucket".into(),
///     path_style: true,
/// };
/// ```
///
/// # AWS S3 example
/// ```rust,ignore
/// let cfg = S3Config {
///     endpoint: None,
///     region: "ap-southeast-1".into(),
///     access_key: Some(std::env::var("AWS_ACCESS_KEY_ID").unwrap()),
///     secret_key: Some(std::env::var("AWS_SECRET_ACCESS_KEY").unwrap()),
///     bucket: "my-bucket".into(),
///     path_style: false,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// Custom endpoint URL. Set to `Some("http://localhost:9000")` for MinIO
    /// or `None` to use the default AWS S3 endpoint for the given region.
    pub endpoint: Option<String>,

    /// AWS region string, e.g. `"us-east-1"`, `"ap-southeast-1"`, or `"auto"`
    /// (Cloudflare R2). Required even when using a custom endpoint.
    pub region: String,

    /// AWS access key ID. If `None`, credentials are resolved from the
    /// environment (AWS_ACCESS_KEY_ID, ~/.aws/credentials, IAM role, etc.).
    pub access_key: Option<String>,

    /// AWS secret access key. If `None`, credentials are resolved from the
    /// environment alongside `access_key`.
    pub secret_key: Option<String>,

    /// Name of the bucket to operate on.
    pub bucket: String,

    /// Use path-style addressing (`endpoint/bucket/key`) instead of virtual-
    /// hosted-style (`bucket.endpoint/key`). **Must be `true` for MinIO.**
    pub path_style: bool,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: None,
            region: "us-east-1".into(),
            access_key: None,
            secret_key: None,
            bucket: String::new(),
            path_style: false,
        }
    }
}
