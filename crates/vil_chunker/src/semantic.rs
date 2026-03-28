use crate::strategy::{estimate_tokens, ChunkMeta, ChunkStrategy, ChunkType, TextChunk};

/// Sentence-boundary chunker that merges consecutive sentences until a token
/// budget is reached.
///
/// Splitting is done on sentence-ending punctuation (`.`, `!`, `?`) followed
/// by whitespace, which covers the vast majority of English prose without
/// pulling in a heavy NLP library.
pub struct SentenceChunker {
    /// Maximum tokens per chunk.
    pub max_tokens: usize,
}

impl SentenceChunker {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }
}

impl ChunkStrategy for SentenceChunker {
    fn chunk(&self, text: &str) -> Vec<TextChunk> {
        if text.is_empty() {
            return Vec::new();
        }

        let sentences = split_sentences(text);
        let mut chunks = Vec::new();
        let mut buf = String::new();
        let mut chunk_start: usize = 0;

        for (sent_start, sent_end, sentence) in &sentences {
            let merged = if buf.is_empty() {
                sentence.to_string()
            } else {
                format!("{} {}", buf.trim_end(), sentence)
            };

            if estimate_tokens(&merged) > self.max_tokens && !buf.is_empty() {
                // Flush current buffer as a chunk.
                let trimmed = buf.trim();
                let token_count = estimate_tokens(trimmed);
                chunks.push(TextChunk {
                    content: trimmed.to_string(),
                    start: chunk_start,
                    end: chunk_start + trimmed.len(),
                    token_count,
                    metadata: ChunkMeta {
                        chunk_type: ChunkType::Sentence,
                        language: None,
                    },
                });
                buf.clear();
                chunk_start = *sent_start;
            }

            if buf.is_empty() {
                buf.push_str(sentence);
                chunk_start = *sent_start;
            } else {
                buf.push(' ');
                buf.push_str(sentence);
            }
            let _ = sent_end; // used implicitly via sentence content
        }

        // Flush remaining buffer.
        if !buf.is_empty() {
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                let token_count = estimate_tokens(trimmed);
                chunks.push(TextChunk {
                    content: trimmed.to_string(),
                    start: chunk_start,
                    end: chunk_start + trimmed.len(),
                    token_count,
                    metadata: ChunkMeta {
                        chunk_type: ChunkType::Sentence,
                        language: None,
                    },
                });
            }
        }

        chunks
    }
}

/// Split text into sentences.
/// Returns `(start_byte, end_byte, &str)` for each sentence.
fn split_sentences(text: &str) -> Vec<(usize, usize, &str)> {
    let mut sentences = Vec::new();
    let mut start = 0;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        // Look for sentence-ending punctuation followed by whitespace or end-of-string.
        if (b == b'.' || b == b'!' || b == b'?')
            && (i + 1 >= len || bytes[i + 1].is_ascii_whitespace())
        {
            let end = i + 1;
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push((start, end, sentence));
            }
            // Skip trailing whitespace.
            i = end;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            start = i;
            continue;
        }
        i += 1;
    }

    // Remaining text that doesn't end with sentence punctuation.
    if start < len {
        let sentence = text[start..].trim();
        if !sentence.is_empty() {
            sentences.push((start, len, sentence));
        }
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_basic_sentences() {
        let chunker = SentenceChunker::new(1000);
        let text = "Hello world. This is a test. Another sentence!";
        let chunks = chunker.chunk(text);
        // With a huge budget, everything merges into one chunk.
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Hello world."));
        assert!(chunks[0].content.contains("Another sentence!"));
    }

    #[test]
    fn splits_when_budget_exceeded() {
        // 5 tokens budget ≈ ~4 words — forces splits.
        let chunker = SentenceChunker::new(5);
        let text = "The quick brown fox. Jumps over the lazy dog. And then some more.";
        let chunks = chunker.chunk(text);
        assert!(
            chunks.len() >= 2,
            "expected >= 2 chunks, got {}",
            chunks.len()
        );
    }

    #[test]
    fn empty_input() {
        let chunker = SentenceChunker::new(100);
        assert!(chunker.chunk("").is_empty());
    }

    #[test]
    fn unicode_content() {
        let chunker = SentenceChunker::new(1000);
        let text = "Hallo Welt. Dies ist ein Test. Schon fertig!";
        let chunks = chunker.chunk(text);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Schon fertig!"));
    }

    #[test]
    fn no_sentence_boundaries() {
        let chunker = SentenceChunker::new(1000);
        let text = "This text has no sentence-ending punctuation";
        let chunks = chunker.chunk(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn token_count_is_positive() {
        let chunker = SentenceChunker::new(1000);
        let chunks = chunker.chunk("Hello world. Goodbye world.");
        for c in &chunks {
            assert!(c.token_count > 0);
        }
    }
}
