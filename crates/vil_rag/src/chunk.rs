use serde::{Deserialize, Serialize};

/// A chunk of text extracted from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk identifier (UUID).
    pub id: String,
    /// Parent document identifier.
    pub doc_id: String,
    /// The chunk text content.
    pub content: String,
    /// Position index within the document.
    pub index: usize,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

/// A chunk with its embedding vector.
#[derive(Debug, Clone)]
pub struct EmbeddedChunk {
    pub chunk: Chunk,
    pub embedding: Vec<f32>,
}

/// Trait for chunking strategies.
pub trait ChunkerStrategy: Send + Sync {
    /// Split text into chunks, tagged with the given document ID.
    fn chunk(&self, doc_id: &str, text: &str) -> Vec<Chunk>;
}

// ---------------------------------------------------------------------------
// FixedChunker — fixed character-count windows with overlap
// ---------------------------------------------------------------------------

/// Chunks text into fixed-size windows with configurable overlap.
pub struct FixedChunker {
    pub chunk_size: usize,
    pub overlap: usize,
}

impl FixedChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
        }
    }
}

impl ChunkerStrategy for FixedChunker {
    fn chunk(&self, doc_id: &str, text: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return chunks;
        }

        let step = if self.chunk_size > self.overlap {
            self.chunk_size - self.overlap
        } else {
            1
        };

        let mut start = 0;
        let mut index = 0;
        while start < len {
            let end = (start + self.chunk_size).min(len);
            let content: String = chars[start..end].iter().collect();
            chunks.push(Chunk {
                id: uuid::Uuid::new_v4().to_string(),
                doc_id: doc_id.to_string(),
                content,
                index,
                metadata: serde_json::json!({}),
            });
            index += 1;
            start += step;
            if end == len {
                break;
            }
        }
        chunks
    }
}

// ---------------------------------------------------------------------------
// SemanticChunker — splits on sentence boundaries, merges until chunk_size
// ---------------------------------------------------------------------------

/// Chunks text by splitting on sentence boundaries (`.`, `!`, `?` followed
/// by whitespace or end-of-string), then merging small sentences until
/// `chunk_size` characters is reached.
pub struct SemanticChunker {
    pub chunk_size: usize,
    pub overlap: usize,
}

impl SemanticChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
        }
    }

    /// Split text into sentences on `.` / `!` / `?` followed by whitespace or EOF.
    fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();

        let mut i = 0;
        while i < len {
            current.push(chars[i]);
            if (chars[i] == '.' || chars[i] == '!' || chars[i] == '?')
                && (i + 1 >= len || chars[i + 1].is_whitespace())
            {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current.clear();
                // skip whitespace after sentence-ending punctuation
                while i + 1 < len && chars[i + 1].is_whitespace() {
                    i += 1;
                }
            }
            i += 1;
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            sentences.push(trimmed);
        }
        sentences
    }
}

impl ChunkerStrategy for SemanticChunker {
    fn chunk(&self, doc_id: &str, text: &str) -> Vec<Chunk> {
        let sentences = Self::split_sentences(text);
        if sentences.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut index = 0;

        for sentence in &sentences {
            let would_be = if current.is_empty() {
                sentence.len()
            } else {
                current.len() + 1 + sentence.len()
            };

            if would_be > self.chunk_size && !current.is_empty() {
                chunks.push(Chunk {
                    id: uuid::Uuid::new_v4().to_string(),
                    doc_id: doc_id.to_string(),
                    content: current.clone(),
                    index,
                    metadata: serde_json::json!({}),
                });
                index += 1;

                // Keep overlap: take the tail of the current chunk
                if self.overlap > 0 && current.len() > self.overlap {
                    current = current[current.len() - self.overlap..].to_string();
                } else if self.overlap == 0 {
                    current.clear();
                }
            }

            if current.is_empty() {
                current = sentence.clone();
            } else {
                current.push(' ');
                current.push_str(sentence);
            }
        }

        if !current.is_empty() {
            chunks.push(Chunk {
                id: uuid::Uuid::new_v4().to_string(),
                doc_id: doc_id.to_string(),
                content: current,
                index,
                metadata: serde_json::json!({}),
            });
        }

        chunks
    }
}

// ---------------------------------------------------------------------------
// MarkdownChunker — splits on `#` headers
// ---------------------------------------------------------------------------

/// Chunks markdown text by splitting on header lines (`# ...`, `## ...`, etc.).
/// Each section becomes a chunk. If a section exceeds `chunk_size`, it is kept
/// as a single chunk (no further splitting).
pub struct MarkdownChunker {
    pub chunk_size: usize,
}

impl MarkdownChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
}

impl ChunkerStrategy for MarkdownChunker {
    fn chunk(&self, doc_id: &str, text: &str) -> Vec<Chunk> {
        let mut sections: Vec<String> = Vec::new();
        let mut current = String::new();

        for line in text.lines() {
            if line.starts_with('#') && !current.is_empty() {
                sections.push(current.trim().to_string());
                current.clear();
            }
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
        if !current.trim().is_empty() {
            sections.push(current.trim().to_string());
        }

        sections
            .into_iter()
            .enumerate()
            .filter(|(_, s)| !s.is_empty())
            .map(|(index, content)| Chunk {
                id: uuid::Uuid::new_v4().to_string(),
                doc_id: doc_id.to_string(),
                content,
                index,
                metadata: serde_json::json!({}),
            })
            .collect()
    }
}

// ==========================================================================
// Tests
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_chunker_basic() {
        let chunker = FixedChunker::new(10, 0);
        let chunks = chunker.chunk("doc1", "Hello world, this is a test string.");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content.len(), 10);
        assert_eq!(chunks[0].doc_id, "doc1");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn fixed_chunker_with_overlap() {
        let chunker = FixedChunker::new(10, 3);
        let chunks = chunker.chunk("doc1", "abcdefghijklmnopqrst"); // 20 chars
                                                                    // step = 10 - 3 = 7, so starts at 0, 7, 14
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "abcdefghij");
        assert_eq!(chunks[1].content, "hijklmnopq");
        assert_eq!(chunks[2].content, "opqrst");
    }

    #[test]
    fn fixed_chunker_empty() {
        let chunker = FixedChunker::new(10, 0);
        let chunks = chunker.chunk("doc1", "");
        assert!(chunks.is_empty());
    }

    #[test]
    fn semantic_chunker_splits_sentences() {
        let chunker = SemanticChunker::new(50, 0);
        let text = "First sentence. Second sentence. Third sentence.";
        let chunks = chunker.chunk("doc1", text);
        // All three sentences fit in 50 chars, so should be one chunk
        assert_eq!(chunks.len(), 1);

        let chunker = SemanticChunker::new(20, 0);
        let chunks = chunker.chunk("doc1", text);
        // Each sentence is ~16 chars, so should split
        assert!(chunks.len() >= 2);
        // First chunk should contain "First sentence."
        assert!(chunks[0].content.contains("First sentence."));
    }

    #[test]
    fn semantic_chunker_exclamation_and_question() {
        let chunker = SemanticChunker::new(30, 0);
        let text = "What is this? It is great! Done.";
        let chunks = chunker.chunk("doc1", text);
        assert!(!chunks.is_empty());
        // All sentences should appear across chunks
        let all: String = chunks
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all.contains("What is this?"));
        assert!(all.contains("It is great!"));
        assert!(all.contains("Done."));
    }

    #[test]
    fn markdown_chunker_splits_on_headers() {
        let chunker = MarkdownChunker::new(1000);
        let text = "# Introduction\nSome intro text.\n## Details\nMore details here.\n## Conclusion\nFinal words.";
        let chunks = chunker.chunk("doc1", text);
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].content.contains("Introduction"));
        assert!(chunks[1].content.contains("Details"));
        assert!(chunks[2].content.contains("Conclusion"));
    }

    #[test]
    fn markdown_chunker_no_headers() {
        let chunker = MarkdownChunker::new(1000);
        let text = "Just plain text without any headers.";
        let chunks = chunker.chunk("doc1", text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }
}
