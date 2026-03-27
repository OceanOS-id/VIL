// Content negotiation: Accept header → format selection.

/// Supported response formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    Json,
    #[cfg(feature = "protobuf")]
    Protobuf,
}

impl ResponseFormat {
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            #[cfg(feature = "protobuf")]
            Self::Protobuf => "application/protobuf",
        }
    }
}

/// Parse Accept header and select best format.
pub fn negotiate(accept: Option<&str>) -> ResponseFormat {
    let _accept = accept.unwrap_or("application/json");

    #[cfg(feature = "protobuf")]
    {
        if accept.contains("application/protobuf") || accept.contains("application/x-protobuf") {
            return ResponseFormat::Protobuf;
        }
    }

    ResponseFormat::Json
}

/// Check if a requested format is supported.
pub fn is_supported(accept: &str) -> bool {
    if accept.contains("application/json") || accept.contains("*/*") {
        return true;
    }
    #[cfg(feature = "protobuf")]
    if accept.contains("application/protobuf") || accept.contains("application/x-protobuf") {
        return true;
    }
    false
}
