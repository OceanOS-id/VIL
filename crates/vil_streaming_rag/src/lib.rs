//! # vil_streaming_rag (D17)
//!
//! Process documents as they stream in — chunking, embedding, and indexing
//! in real-time. Unlike batch RAG pipelines, this crate handles continuous
//! data feeds where documents arrive incrementally.
//!
//! ## Quick start
//!
//! ```rust
//! use vil_streaming_rag::{StreamingIngester, StreamConfig};
//!
//! let ingester = StreamingIngester::new(StreamConfig {
//!     chunk_size: 100,
//!     overlap: 10,
//!     flush_interval_ms: 0,
//!     max_buffer_size: 4096,
//! });
//!
//! ingester.ingest_chunk("Hello world. This is a streaming document ");
//! ingester.ingest_chunk("that arrives in pieces over time.");
//! ingester.flush();
//!
//! assert!(ingester.chunk_count() > 0);
//! ```

pub mod config;
pub mod handlers;
pub mod index_writer;
pub mod pipeline_sse;
pub mod plugin;
pub mod processor;
pub mod semantic;
pub mod stream;

pub use config::{StreamConfig, StreamConfigBuilder};
pub use index_writer::{IndexWriter, IndexedChunk, StreamResult};
pub use plugin::StreamingRagPlugin;
pub use processor::StreamProcessor;
pub use semantic::{StreamingRagEvent, StreamingRagFault, StreamingRagState};
pub use stream::{compute_embedding, StreamingIngester};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_small_text() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 100,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        ingester.ingest_chunk("short text");
        // Not enough to fill a chunk, stays in buffer
        assert_eq!(ingester.chunk_count(), 0);
        assert!(ingester.buffer_len() > 0);
        // Flush to index it
        ingester.flush();
        assert_eq!(ingester.chunk_count(), 1);
    }

    #[test]
    fn test_ingest_large_text_produces_multiple_chunks() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 10,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        // 50 characters -> should produce 5 chunks of 10
        ingester.ingest_chunk("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWX");
        assert_eq!(ingester.chunk_count(), 5);
        assert_eq!(ingester.buffer_len(), 0);
    }

    #[test]
    fn test_flush_remaining() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 20,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        ingester.ingest_chunk("hello world"); // 11 chars < 20
        assert_eq!(ingester.chunk_count(), 0);
        ingester.flush();
        assert_eq!(ingester.chunk_count(), 1);
    }

    #[test]
    fn test_search_after_ingest() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 10,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        ingester.ingest_chunk("aaaaaaaaaa"); // 10 a's -> one chunk
        ingester.ingest_chunk("bbbbbbbbbb"); // 10 b's -> one chunk

        // Build a query embedding similar to 'a' text
        let query = compute_embedding("aaaaaaaaaa");
        let results = ingester.search(&query, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "aaaaaaaaaa");
    }

    #[test]
    fn test_empty_search() {
        let ingester = StreamingIngester::with_defaults();
        let query = vec![0.0f32; 128];
        let results = ingester.search(&query, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_incremental_ingest() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 5,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        // Feed one char at a time
        for c in "abcdefghij".chars() {
            ingester.ingest_chunk(&c.to_string());
        }
        // 10 chars / 5 chunk_size = 2 chunks
        assert_eq!(ingester.chunk_count(), 2);
    }

    #[test]
    fn test_config_builder() {
        let config = StreamConfigBuilder::new()
            .chunk_size(256)
            .overlap(32)
            .flush_interval_ms(500)
            .max_buffer_size(2048)
            .build();

        assert_eq!(config.chunk_size, 256);
        assert_eq!(config.overlap, 32);
        assert_eq!(config.flush_interval_ms, 500);
        assert_eq!(config.max_buffer_size, 2048);
    }

    #[test]
    fn test_concurrent_ingest() {
        use std::sync::Arc;
        use std::thread;

        let ingester = Arc::new(StreamingIngester::new(StreamConfig {
            chunk_size: 5,
            overlap: 0,
            flush_interval_ms: 0,
            max_buffer_size: 40960,
        }));

        let mut handles = Vec::new();
        for i in 0..4 {
            let ing = Arc::clone(&ingester);
            handles.push(thread::spawn(move || {
                let text = format!("{}", (b'a' + i as u8) as char).repeat(20);
                ing.ingest_chunk(&text);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        ingester.flush();
        // Each thread sends 20 chars / 5 chunk_size = 4 chunks, but interleaving
        // means we can't predict exact count — just ensure nothing crashed and
        // we have a reasonable number.
        assert!(ingester.chunk_count() >= 4);
    }

    #[test]
    fn test_overlap_produces_overlapping_content() {
        let ingester = StreamingIngester::new(StreamConfig {
            chunk_size: 10,
            overlap: 5,
            flush_interval_ms: 0,
            max_buffer_size: 4096,
        });
        // 20 chars with chunk_size=10, overlap=5 -> advance 5 each time
        // chunks: [0..10], [5..15], [10..20]
        ingester.ingest_chunk("01234567890123456789");
        ingester.flush();
        assert!(ingester.chunk_count() >= 2);
    }
}
