use crate::strategy::{estimate_tokens, ChunkMeta, ChunkStrategy, ChunkType, TextChunk};

/// Sliding-window chunker with configurable window size and overlap.
///
/// Splits text into word-based windows of `window_size` words, advancing by
/// `window_size - overlap` words each step.
pub struct SlidingWindowChunker {
    /// Number of words per window.
    pub window_size: usize,
    /// Number of overlapping words between consecutive windows.
    pub overlap: usize,
}

impl SlidingWindowChunker {
    pub fn new(window_size: usize, overlap: usize) -> Self {
        assert!(window_size > 0, "window_size must be > 0");
        assert!(overlap < window_size, "overlap must be < window_size");
        Self {
            window_size,
            overlap,
        }
    }
}

impl ChunkStrategy for SlidingWindowChunker {
    fn chunk(&self, text: &str) -> Vec<TextChunk> {
        if text.is_empty() {
            return Vec::new();
        }

        // Collect word spans (byte start, byte end).
        let word_spans: Vec<(usize, usize)> = {
            let mut spans = Vec::new();
            let mut in_word = false;
            let mut start = 0;
            for (i, ch) in text.char_indices() {
                if ch.is_whitespace() {
                    if in_word {
                        spans.push((start, i));
                        in_word = false;
                    }
                } else if !in_word {
                    start = i;
                    in_word = true;
                }
            }
            if in_word {
                spans.push((start, text.len()));
            }
            spans
        };

        if word_spans.is_empty() {
            return Vec::new();
        }

        let step = self.window_size - self.overlap;
        let mut chunks = Vec::new();
        let mut pos = 0;

        while pos < word_spans.len() {
            let end = (pos + self.window_size).min(word_spans.len());
            let byte_start = word_spans[pos].0;
            let byte_end = word_spans[end - 1].1;
            let content = &text[byte_start..byte_end];
            let token_count = estimate_tokens(content);

            chunks.push(TextChunk {
                content: content.to_string(),
                start: byte_start,
                end: byte_end,
                token_count,
                metadata: ChunkMeta {
                    chunk_type: ChunkType::Paragraph,
                    language: None,
                },
            });

            if end >= word_spans.len() {
                break;
            }
            pos += step;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_sliding_window() {
        let chunker = SlidingWindowChunker::new(4, 1);
        let text = "one two three four five six seven";
        let chunks = chunker.chunk(text);
        assert!(chunks.len() >= 2);
        // First window: "one two three four"
        assert_eq!(chunks[0].content, "one two three four");
    }

    #[test]
    fn overlap_shares_words() {
        let chunker = SlidingWindowChunker::new(3, 1);
        let text = "a b c d e";
        let chunks = chunker.chunk(text);
        // Window 1: a b c, Window 2: c d e
        assert!(chunks.len() >= 2);
        // Last word of chunk 0 should appear in chunk 1
        let last_word_0 = chunks[0].content.split_whitespace().last().unwrap();
        let first_word_1 = chunks[1].content.split_whitespace().next().unwrap();
        assert_eq!(last_word_0, first_word_1);
    }

    #[test]
    fn empty_input() {
        let chunker = SlidingWindowChunker::new(5, 2);
        assert!(chunker.chunk("").is_empty());
    }

    #[test]
    fn input_shorter_than_window() {
        let chunker = SlidingWindowChunker::new(10, 3);
        let chunks = chunker.chunk("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello world");
    }

    #[test]
    fn whitespace_only() {
        let chunker = SlidingWindowChunker::new(5, 2);
        assert!(chunker.chunk("   \t\n  ").is_empty());
    }

    #[test]
    #[should_panic]
    fn overlap_exceeds_window() {
        SlidingWindowChunker::new(3, 3);
    }
}
