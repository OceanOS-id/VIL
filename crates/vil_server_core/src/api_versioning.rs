// =============================================================================
// VIL Server — API Versioning
// =============================================================================
//
// Supports multiple API versioning strategies:
//   - URL path:  /v1/users, /v2/users
//   - Header:    X-API-Version: 2
//   - Accept:    Accept: application/vnd.vil.v2+json
//
// The version extractor resolves the requested version and injects
// it into the handler context.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde::Serialize;

/// Resolved API version.
#[derive(Debug, Clone, Serialize)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub source: VersionSource,
}

/// How the version was determined.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum VersionSource {
    UrlPath,
    Header,
    Accept,
    Default,
}

impl ApiVersion {
    pub fn new(major: u32, minor: u32, source: VersionSource) -> Self {
        Self {
            major,
            minor,
            source,
        }
    }

    pub fn v1() -> Self {
        Self::new(1, 0, VersionSource::Default)
    }
    pub fn v2() -> Self {
        Self::new(2, 0, VersionSource::Default)
    }

    pub fn is_v1(&self) -> bool {
        self.major == 1
    }
    pub fn is_v2(&self) -> bool {
        self.major == 2
    }
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

/// Axum extractor for API version.
///
/// Resolution order:
/// 1. URL path prefix (/v1/, /v2/)
/// 2. X-API-Version header
/// 3. Accept header (vnd.vil.v2+json)
/// 4. Default (v1)
#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for ApiVersion {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let path = parts.uri.path();

        // 1. URL path prefix
        if let Some(version) = extract_from_path(path) {
            return Ok(version);
        }

        // 2. X-API-Version header
        if let Some(header) = parts.headers.get("x-api-version") {
            if let Ok(val) = header.to_str() {
                if let Ok(major) = val.parse::<u32>() {
                    return Ok(ApiVersion::new(major, 0, VersionSource::Header));
                }
            }
        }

        // 3. Accept header (vnd.vil.vN)
        if let Some(accept) = parts.headers.get("accept") {
            if let Ok(val) = accept.to_str() {
                if let Some(version) = extract_from_accept(val) {
                    return Ok(version);
                }
            }
        }

        // 4. Default
        Ok(ApiVersion::v1())
    }
}

fn extract_from_path(path: &str) -> Option<ApiVersion> {
    let segments: Vec<&str> = path.split('/').collect();
    for seg in &segments {
        if let Some(rest) = seg.strip_prefix('v') {
            if let Ok(major) = rest.parse::<u32>() {
                return Some(ApiVersion::new(major, 0, VersionSource::UrlPath));
            }
        }
    }
    None
}

fn extract_from_accept(accept: &str) -> Option<ApiVersion> {
    // Look for vnd.vil.v{N}
    if let Some(idx) = accept.find("vnd.vil.v") {
        let rest = &accept[idx + 11..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(major) = num_str.parse::<u32>() {
            return Some(ApiVersion::new(major, 0, VersionSource::Accept));
        }
    }
    None
}
