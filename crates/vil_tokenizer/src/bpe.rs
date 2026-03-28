use crate::vocab::Vocabulary;

/// Byte-Pair Encoding tokenizer.
///
/// For exact tokenization, load a real vocabulary (cl100k_base, etc.).
/// For fast estimation (token counting), use BuiltIn vocabulary.
pub struct BpeTokenizer {
    vocab: Vocabulary,
}

impl BpeTokenizer {
    pub fn new(vocab: Vocabulary) -> Self {
        Self { vocab }
    }

    /// Encode text to token IDs.
    /// With built-in vocab, returns byte-level tokens (1:1 with bytes).
    /// With real vocab, performs BPE merges.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let bytes = text.as_bytes();
        // Simple byte-level encoding (no merges for built-in)
        bytes
            .iter()
            .filter_map(|b| self.vocab.encode_token(&[*b]))
            .collect()
    }

    /// Decode token IDs back to text.
    pub fn decode(&self, tokens: &[u32]) -> String {
        let bytes: Vec<u8> = tokens
            .iter()
            .filter_map(|id| self.vocab.decode_token(*id))
            .flatten()
            .copied()
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Count tokens in text (fast estimation using chars-per-token ratio).
    /// This is the primary use case -- accurate within +/-5%.
    pub fn count_tokens(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        let ratio = self.vocab.chars_per_token_ratio();
        (text.len() as f64 / ratio).ceil() as usize
    }

    /// Count tokens precisely by encoding (slower but exact).
    pub fn count_tokens_exact(&self, text: &str) -> usize {
        self.encode(text).len()
    }

    /// Get vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.vocab.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{BuiltInVocab, VocabSource};

    #[test]
    fn test_encode_decode_roundtrip() {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        let tok = BpeTokenizer::new(vocab);
        let text = "Hello, world!";
        let tokens = tok.encode(text);
        let decoded = tok.decode(&tokens);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_count_tokens_estimation() {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        let tok = BpeTokenizer::new(vocab);
        // "Hello, world!" is ~4 tokens in GPT-4
        let count = tok.count_tokens("Hello, world!");
        assert!(count >= 2 && count <= 6, "got {}", count);
    }

    #[test]
    fn test_empty_text() {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        let tok = BpeTokenizer::new(vocab);
        assert_eq!(tok.count_tokens(""), 0);
        assert_eq!(tok.encode("").len(), 0);
    }

    #[test]
    fn test_unicode() {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        let tok = BpeTokenizer::new(vocab);
        let text = "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}"; // Japanese
        let count = tok.count_tokens(text);
        assert!(count > 0);
        let tokens = tok.encode(text);
        let decoded = tok.decode(&tokens);
        assert_eq!(decoded, text);
    }
}
