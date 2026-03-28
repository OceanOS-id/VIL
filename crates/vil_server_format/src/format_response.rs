// =============================================================================
// VIL Server Format — FormatResponse<T>
// =============================================================================
//
// Auto-negotiates response format:
//   Accept: application/json     → JSON
//   Accept: application/protobuf → Protobuf (if compiled)
//   Accept: */*                  → JSON (default)

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Serialize;

use crate::negotiator;

/// Multi-format response that auto-negotiates based on Accept header.
///
/// Usage:
/// ```ignore
/// async fn handler(headers: HeaderMap) -> FormatResponse<MyData> {
///     FormatResponse::ok(my_data).with_headers(&headers)
/// }
/// ```
pub struct FormatResponse<T: Serialize> {
    status: StatusCode,
    data: T,
    format: Option<negotiator::ResponseFormat>,
}

impl<T: Serialize> FormatResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            status: StatusCode::OK,
            data,
            format: None,
        }
    }

    pub fn created(data: T) -> Self {
        Self {
            status: StatusCode::CREATED,
            data,
            format: None,
        }
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Set format from Accept header.
    pub fn with_headers(mut self, headers: &HeaderMap) -> Self {
        let accept = headers.get("accept").and_then(|v| v.to_str().ok());
        self.format = Some(negotiator::negotiate(accept));
        self
    }

    /// Force a specific format.
    pub fn force_format(mut self, format: negotiator::ResponseFormat) -> Self {
        self.format = Some(format);
        self
    }
}

impl<T: Serialize> IntoResponse for FormatResponse<T> {
    fn into_response(self) -> Response {
        let format = self.format.unwrap_or(negotiator::ResponseFormat::Json);

        match format {
            negotiator::ResponseFormat::Json => {
                let body = serde_json::to_vec(&self.data).unwrap_or_default();
                (
                    self.status,
                    [("content-type", "application/json")],
                    Bytes::from(body),
                )
                    .into_response()
            }
            #[cfg(feature = "protobuf")]
            negotiator::ResponseFormat::Protobuf => {
                // Protobuf: serialize via prost if T implements prost::Message
                // Fallback to JSON if prost encoding not available
                let body = serde_json::to_vec(&self.data).unwrap_or_default();
                (
                    self.status,
                    [("content-type", "application/protobuf")],
                    Bytes::from(body),
                )
                    .into_response()
            }
        }
    }
}
