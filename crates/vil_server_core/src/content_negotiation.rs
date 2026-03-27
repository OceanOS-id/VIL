// =============================================================================
// VIL Server — Content Negotiation
// =============================================================================
//
// Parses Accept header and determines the best response content type.
// Supports: application/json, text/plain, text/html, application/xml
//
// Usage:
//   async fn handler(accept: AcceptHeader) -> impl IntoResponse {
//       match accept.preferred() {
//           ContentType::Json => Json(data).into_response(),
//           ContentType::Plain => data.to_string().into_response(),
//           _ => Json(data).into_response(), // default
//       }
//   }

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

/// Parsed Accept header with content type preferences.
#[derive(Debug, Clone)]
pub struct AcceptHeader {
    types: Vec<(ContentType, f32)>,
}

/// Supported content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Json,
    Plain,
    Html,
    Xml,
    OctetStream,
    Any,
}

impl ContentType {
    pub fn mime(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Plain => "text/plain",
            Self::Html => "text/html",
            Self::Xml => "application/xml",
            Self::OctetStream => "application/octet-stream",
            Self::Any => "*/*",
        }
    }

    fn from_mime(mime: &str) -> Option<Self> {
        match mime.trim() {
            "application/json" => Some(Self::Json),
            "text/plain" => Some(Self::Plain),
            "text/html" => Some(Self::Html),
            "application/xml" | "text/xml" => Some(Self::Xml),
            "application/octet-stream" => Some(Self::OctetStream),
            "*/*" => Some(Self::Any),
            _ => None,
        }
    }
}

impl AcceptHeader {
    /// Get the most preferred content type.
    pub fn preferred(&self) -> ContentType {
        self.types.first().map(|(ct, _)| *ct).unwrap_or(ContentType::Json)
    }

    /// Check if a specific content type is accepted.
    pub fn accepts(&self, ct: ContentType) -> bool {
        self.types.iter().any(|(t, _)| *t == ct || *t == ContentType::Any)
    }

    /// Check if JSON is preferred.
    pub fn wants_json(&self) -> bool {
        matches!(self.preferred(), ContentType::Json | ContentType::Any)
    }
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AcceptHeader {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let accept = parts
            .headers
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json");

        let mut types: Vec<(ContentType, f32)> = accept
            .split(',')
            .filter_map(|part| {
                let part = part.trim();
                let (mime, quality) = if let Some((m, q)) = part.split_once(";q=") {
                    (m.trim(), q.trim().parse().unwrap_or(1.0))
                } else {
                    (part, 1.0f32)
                };
                ContentType::from_mime(mime).map(|ct| (ct, quality))
            })
            .collect();

        types.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(AcceptHeader { types })
    }
}
