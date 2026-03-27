use crate::config::StreamConfig;

/// Accumulates incoming text and emits fixed-size chunks with optional overlap.
pub struct StreamProcessor {
    buffer: String,
    config: StreamConfig,
}

impl StreamProcessor {
    pub fn new(config: StreamConfig) -> Self {
        Self {
            buffer: String::new(),
            config,
        }
    }

    /// Append text to the internal buffer and return any complete chunks.
    pub fn push(&mut self, text: &str) -> Vec<String> {
        self.buffer.push_str(text);
        self.drain_chunks()
    }

    /// Force-drain remaining buffer content as a final chunk (may be smaller than chunk_size).
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            let chunk = self.buffer.clone();
            self.buffer.clear();
            Some(chunk)
        }
    }

    /// Returns current buffer length.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the buffer exceeds `max_buffer_size`.
    pub fn should_force_flush(&self) -> bool {
        self.buffer.len() >= self.config.max_buffer_size
    }

    fn drain_chunks(&mut self) -> Vec<String> {
        let mut chunks = Vec::new();
        while self.buffer.len() >= self.config.chunk_size {
            let chunk: String = self.buffer.chars().take(self.config.chunk_size).collect();
            let chunk_byte_len = chunk.len();
            chunks.push(chunk);

            // Advance buffer by chunk_size - overlap
            let advance = self.config.chunk_size.saturating_sub(self.config.overlap);
            let advance = advance.min(chunk_byte_len);
            // Work with char boundary
            let mut byte_advance = 0;
            for (i, (idx, _)) in self.buffer.char_indices().enumerate() {
                if i == advance {
                    byte_advance = idx;
                    break;
                }
                byte_advance = idx + self.buffer[idx..].chars().next().map_or(0, |c| c.len_utf8());
            }
            self.buffer = self.buffer[byte_advance..].to_string();
        }
        chunks
    }
}
