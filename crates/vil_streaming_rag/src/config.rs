use serde::{Deserialize, Serialize};

/// Configuration for the streaming RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Number of characters per chunk.
    pub chunk_size: usize,
    /// Number of overlapping characters between consecutive chunks.
    pub overlap: usize,
    /// How often (in ms) to auto-flush the buffer. 0 = manual only.
    pub flush_interval_ms: u64,
    /// Maximum buffer size in characters before forcing a flush.
    pub max_buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            overlap: 64,
            flush_interval_ms: 1000,
            max_buffer_size: 4096,
        }
    }
}

/// Builder for `StreamConfig`.
pub struct StreamConfigBuilder {
    config: StreamConfig,
}

impl StreamConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: StreamConfig::default(),
        }
    }

    pub fn chunk_size(mut self, size: usize) -> Self {
        self.config.chunk_size = size;
        self
    }

    pub fn overlap(mut self, overlap: usize) -> Self {
        self.config.overlap = overlap;
        self
    }

    pub fn flush_interval_ms(mut self, ms: u64) -> Self {
        self.config.flush_interval_ms = ms;
        self
    }

    pub fn max_buffer_size(mut self, size: usize) -> Self {
        self.config.max_buffer_size = size;
        self
    }

    pub fn build(self) -> StreamConfig {
        self.config
    }
}

impl Default for StreamConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
