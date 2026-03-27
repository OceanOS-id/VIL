// =============================================================================
// VX Descriptor — VASI-compliant fixed-size request/response descriptors
// =============================================================================
//
// All fields are fixed-size primitives (u64, u32, u16, u8).
// No String, no Vec — fully VASI-compliant for SHM transport.
//
// These descriptors travel through the Trigger Lane as lightweight
// envelopes. The actual body lives in the Data Lane (SHM region).

/// HTTP method encoded as a single byte for SHM transport.
pub const METHOD_GET: u8 = 0;
pub const METHOD_POST: u8 = 1;
pub const METHOD_PUT: u8 = 2;
pub const METHOD_DELETE: u8 = 3;
pub const METHOD_PATCH: u8 = 4;
pub const METHOD_HEAD: u8 = 5;
pub const METHOD_OPTIONS: u8 = 6;

/// Content-type encoded as a single byte for SHM transport.
pub const CONTENT_TYPE_NONE: u8 = 0;
pub const CONTENT_TYPE_JSON: u8 = 1;
pub const CONTENT_TYPE_FORM: u8 = 2;
pub const CONTENT_TYPE_MULTIPART: u8 = 3;
pub const CONTENT_TYPE_OCTET_STREAM: u8 = 4;
pub const CONTENT_TYPE_TEXT: u8 = 5;
pub const CONTENT_TYPE_XML: u8 = 6;

/// Compact VASI-compliant descriptor for the Trigger Lane.
///
/// Describes an inbound HTTP request without any heap allocation.
/// The body itself lives in SHM at `body_offset..body_offset+body_len`.
///
/// Size: 40 bytes (fits in a single cache line on most architectures).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RequestDescriptor {
    /// Unique request identifier (monotonic counter or UUID-derived)
    pub request_id: u64,
    /// HTTP method (see METHOD_* constants)
    pub method: u8,
    /// Content-Type of the body (see CONTENT_TYPE_* constants)
    pub content_type: u8,
    /// Reserved flags (keep-alive, upgrade, etc.)
    pub flags: u16,
    /// Endpoint identifier — maps to a registered handler
    pub endpoint_id: u32,
    /// SHM region offset where the request body starts
    pub body_offset: u64,
    /// Length of the request body in bytes
    pub body_len: u32,
    /// Query string offset in SHM (0 = no query)
    pub query_offset: u64,
    /// Query string length
    pub query_len: u16,
    /// Padding to align struct
    pub _pad: u16,
}

impl RequestDescriptor {
    /// Create a new request descriptor.
    pub fn new(request_id: u64, method: u8, endpoint_id: u32) -> Self {
        Self {
            request_id,
            method,
            content_type: CONTENT_TYPE_NONE,
            flags: 0,
            endpoint_id,
            body_offset: 0,
            body_len: 0,
            query_offset: 0,
            query_len: 0,
            _pad: 0,
        }
    }

    /// Set the body location in SHM.
    pub fn with_body(mut self, offset: u64, len: u32, content_type: u8) -> Self {
        self.body_offset = offset;
        self.body_len = len;
        self.content_type = content_type;
        self
    }

    /// Set the query string location in SHM.
    pub fn with_query(mut self, offset: u64, len: u16) -> Self {
        self.query_offset = offset;
        self.query_len = len;
        self
    }

    /// Return the HTTP method as a string.
    pub fn method_str(&self) -> &'static str {
        match self.method {
            METHOD_GET => "GET",
            METHOD_POST => "POST",
            METHOD_PUT => "PUT",
            METHOD_DELETE => "DELETE",
            METHOD_PATCH => "PATCH",
            METHOD_HEAD => "HEAD",
            METHOD_OPTIONS => "OPTIONS",
            _ => "UNKNOWN",
        }
    }

    /// Check if the body content type is JSON.
    pub fn is_json(&self) -> bool {
        self.content_type == CONTENT_TYPE_JSON
    }

    /// Check if the body content type is form-encoded.
    pub fn is_form(&self) -> bool {
        self.content_type == CONTENT_TYPE_FORM
    }

    /// Check if the request has a body.
    pub fn has_body(&self) -> bool {
        self.body_len > 0
    }

    /// Check if the request has a query string.
    pub fn has_query(&self) -> bool {
        self.query_len > 0
    }

    /// Convert an `axum::http::Method` to our u8 encoding.
    pub fn encode_method(method: &axum::http::Method) -> u8 {
        match *method {
            axum::http::Method::GET => METHOD_GET,
            axum::http::Method::POST => METHOD_POST,
            axum::http::Method::PUT => METHOD_PUT,
            axum::http::Method::DELETE => METHOD_DELETE,
            axum::http::Method::PATCH => METHOD_PATCH,
            axum::http::Method::HEAD => METHOD_HEAD,
            axum::http::Method::OPTIONS => METHOD_OPTIONS,
            _ => 0xFF,
        }
    }
}

/// Compact VASI-compliant descriptor for the response path.
///
/// Describes an outbound HTTP response. The body lives in SHM.
///
/// Size: 24 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ResponseDescriptor {
    /// Request ID this response belongs to
    pub request_id: u64,
    /// HTTP status code
    pub status_code: u16,
    /// Content-Type of the response body
    pub content_type: u8,
    /// Reserved flags (chunked, compressed, etc.)
    pub flags: u8,
    /// Length of the response body in bytes
    pub body_len: u32,
    /// SHM region offset where the response body starts
    pub body_offset: u64,
}

impl ResponseDescriptor {
    /// Create a new response descriptor.
    pub fn new(request_id: u64, status_code: u16) -> Self {
        Self {
            request_id,
            status_code,
            content_type: CONTENT_TYPE_NONE,
            flags: 0,
            body_len: 0,
            body_offset: 0,
        }
    }

    /// Set the body location in SHM.
    pub fn with_body(mut self, offset: u64, len: u32, content_type: u8) -> Self {
        self.body_offset = offset;
        self.body_len = len;
        self.content_type = content_type;
        self
    }

    /// Check if the response indicates success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Check if the response has a body.
    pub fn has_body(&self) -> bool {
        self.body_len > 0
    }

    /// Check if the response body is JSON.
    pub fn is_json(&self) -> bool {
        self.content_type == CONTENT_TYPE_JSON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_descriptor_basic() {
        let desc = RequestDescriptor::new(42, METHOD_POST, 7)
            .with_body(1024, 256, CONTENT_TYPE_JSON)
            .with_query(2048, 32);

        assert_eq!(desc.request_id, 42);
        assert_eq!(desc.method_str(), "POST");
        assert!(desc.is_json());
        assert!(desc.has_body());
        assert!(desc.has_query());
        assert_eq!(desc.body_offset, 1024);
        assert_eq!(desc.body_len, 256);
    }

    #[test]
    fn response_descriptor_basic() {
        let desc = ResponseDescriptor::new(42, 200)
            .with_body(4096, 128, CONTENT_TYPE_JSON);

        assert!(desc.is_success());
        assert!(desc.has_body());
        assert!(desc.is_json());
    }

    #[test]
    fn request_descriptor_is_copy() {
        let desc = RequestDescriptor::new(1, METHOD_GET, 0);
        let copy = desc;
        assert_eq!(desc.request_id, copy.request_id);
    }
}
