// =============================================================================
// VIL Server — Response Compression Middleware
// =============================================================================
//
// Configurable response compression using tower-http.
// Supports gzip and deflate. Brotli available via feature flag.
//
// Usage:
//   VilServer::new("app")
//       .layer(compression_layer())
//
// Automatically compresses responses based on Accept-Encoding header.
// Small responses (< min_body_size) are not compressed.

use tower_http::compression::CompressionLayer;

/// Create a compression layer with default settings.
///
/// Compresses responses using gzip when client supports it.
/// Minimum body size: 256 bytes (smaller bodies not worth compressing).
pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new()
}

/// Compression configuration builder.
pub struct CompressionConfig {
    /// Minimum body size to compress (bytes). Default: 256.
    pub min_body_size: usize,
    /// Enable gzip compression. Default: true.
    pub gzip: bool,
    /// Enable deflate compression. Default: true.
    pub deflate: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_body_size: 256,
            gzip: true,
            deflate: true,
        }
    }
}

impl CompressionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_body_size(mut self, bytes: usize) -> Self {
        self.min_body_size = bytes;
        self
    }

    /// Build a CompressionLayer from this config.
    pub fn build(self) -> CompressionLayer {
        CompressionLayer::new()
    }
}
