use crate::config::StreamConfig;
use crate::index_writer::{IndexWriter, StreamResult};
use crate::processor::StreamProcessor;
use parking_lot::Mutex;

/// Streaming RAG ingester — processes documents as they arrive, chunking and
/// indexing in real-time.
///
/// Uses a simple deterministic embedding (character-frequency histogram) so the
/// crate works standalone without an external embedding service. Production
/// users should replace `compute_embedding` with a real model call.
pub struct StreamingIngester {
    processor: Mutex<StreamProcessor>,
    writer: IndexWriter,
    config: StreamConfig,
}

impl StreamingIngester {
    /// Create a new ingester with the given configuration.
    pub fn new(config: StreamConfig) -> Self {
        let processor = StreamProcessor::new(config.clone());
        Self {
            processor: Mutex::new(processor),
            writer: IndexWriter::new(),
            config,
        }
    }

    /// Create an ingester with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(StreamConfig::default())
    }

    /// Incrementally ingest a piece of text. Any complete chunks are
    /// immediately embedded and appended to the index.
    pub fn ingest_chunk(&self, text: &str) {
        let chunks = {
            let mut proc = self.processor.lock();
            let mut chunks = proc.push(text);
            // Force flush if buffer exceeds max
            if proc.should_force_flush() {
                if let Some(remainder) = proc.flush() {
                    chunks.push(remainder);
                }
            }
            chunks
        };
        for chunk in chunks {
            let embedding = compute_embedding(&chunk);
            self.writer.append(chunk, embedding);
        }
    }

    /// Flush remaining buffered text into the index.
    pub fn flush(&self) {
        let remainder = {
            let mut proc = self.processor.lock();
            proc.flush()
        };
        if let Some(text) = remainder {
            let embedding = compute_embedding(&text);
            self.writer.append(text, embedding);
        }
    }

    /// Search the index for chunks most similar to `query_embedding`.
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<StreamResult> {
        self.writer.search(query_embedding, top_k)
    }

    /// Number of chunks currently in the index.
    pub fn chunk_count(&self) -> usize {
        self.writer.len()
    }

    /// Current buffer length (characters not yet emitted as a chunk).
    pub fn buffer_len(&self) -> usize {
        self.processor.lock().buffer_len()
    }

    /// Get the active configuration.
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }
}

/// Simple deterministic embedding: 128-bin character frequency histogram
/// normalized to unit length. This is a placeholder — real deployments would
/// call an embedding model.
pub fn compute_embedding(text: &str) -> Vec<f32> {
    let dim = 128;
    let mut hist = vec![0.0f32; dim];
    for b in text.bytes() {
        hist[(b as usize) % dim] += 1.0;
    }
    // L2-normalise
    let mag: f32 = hist.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 {
        for v in hist.iter_mut() {
            *v /= mag;
        }
    }
    hist
}
