use serde::{Deserialize, Serialize};

/// The type of content a chunk contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkType {
    /// A paragraph of prose.
    Paragraph,
    /// One or more sentences.
    Sentence,
    /// A code block or function body.
    Code,
    /// A table or CSV fragment.
    Table,
    /// A heading / title.
    Header,
}

/// Metadata attached to every chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// What kind of content this chunk contains.
    pub chunk_type: ChunkType,
    /// Programming language (if `chunk_type == Code`).
    pub language: Option<String>,
}

/// A single chunk of text produced by a `ChunkStrategy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// The chunk content.
    pub content: String,
    /// Byte offset of the start of this chunk in the original text.
    pub start: usize,
    /// Byte offset of the end of this chunk in the original text.
    pub end: usize,
    /// Approximate token count (estimated as `word_count * 1.3`).
    pub token_count: usize,
    /// Chunk metadata.
    pub metadata: ChunkMeta,
}

/// Trait that all chunking strategies implement.
pub trait ChunkStrategy: Send + Sync {
    /// Split `text` into chunks.
    fn chunk(&self, text: &str) -> Vec<TextChunk>;
}

/// Estimate token count from a text slice using word-count heuristic.
///
/// This avoids pulling in heavy tokenizer machinery for the default path.
/// Callers that need exact counts can use `vil_tokenizer` directly.
pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    // GPT-family models average ~1.3 tokens per word for English.
    ((words as f64) * 1.3).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_single_word() {
        assert!(estimate_tokens("hello") >= 1);
    }

    #[test]
    fn estimate_tokens_sentence() {
        let est = estimate_tokens("The quick brown fox jumps over the lazy dog.");
        // 9 words * 1.3 ≈ 12
        assert!(est >= 9 && est <= 20);
    }

    #[test]
    fn chunk_type_serde_roundtrip() {
        let ct = ChunkType::Code;
        let json = serde_json::to_string(&ct).unwrap();
        let back: ChunkType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ChunkType::Code);
    }
}
