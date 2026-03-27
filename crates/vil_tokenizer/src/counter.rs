use crate::bpe::BpeTokenizer;
use crate::vocab::{Vocabulary, VocabSource, BuiltInVocab};

/// High-level token counter for common LLM models.
pub struct TokenCounter {
    tokenizer: BpeTokenizer,
    model_name: String,
}

impl TokenCounter {
    /// Create a counter for GPT-4/GPT-4o (cl100k_base).
    pub fn gpt4() -> Self {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        Self { tokenizer: BpeTokenizer::new(vocab), model_name: "gpt-4".into() }
    }

    /// Create a counter for GPT-3.5 (p50k_base).
    pub fn gpt35() -> Self {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::P50kBase)).unwrap();
        Self { tokenizer: BpeTokenizer::new(vocab), model_name: "gpt-3.5".into() }
    }

    /// Create a counter for Llama models.
    pub fn llama() -> Self {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Llama)).unwrap();
        Self { tokenizer: BpeTokenizer::new(vocab), model_name: "llama".into() }
    }

    /// Create a counter from a custom vocabulary file.
    pub fn from_vocab_file(path: &str, model_name: &str) -> Result<Self, crate::vocab::VocabError> {
        let vocab = Vocabulary::load(VocabSource::JsonFile(path.into()))?;
        Ok(Self { tokenizer: BpeTokenizer::new(vocab), model_name: model_name.into() })
    }

    /// Count tokens in text.
    pub fn count(&self, text: &str) -> usize {
        self.tokenizer.count_tokens(text)
    }

    /// Count tokens for a chat message (includes role overhead).
    /// OpenAI adds ~4 tokens per message for formatting.
    pub fn count_message(&self, role: &str, content: &str) -> usize {
        let overhead = 4; // <|im_start|>role\ncontent<|im_end|>
        self.tokenizer.count_tokens(role) + self.tokenizer.count_tokens(content) + overhead
    }

    /// Count tokens for a list of chat messages.
    pub fn count_messages(&self, messages: &[(String, String)]) -> usize {
        let base_overhead = 3; // every reply has <|start|>assistant<|message|>
        messages.iter().map(|(role, content)| self.count_message(role, content)).sum::<usize>() + base_overhead
    }

    /// Check if text fits within a token budget.
    pub fn fits(&self, text: &str, max_tokens: usize) -> bool {
        self.count(text) <= max_tokens
    }

    /// Model name.
    pub fn model(&self) -> &str { &self.model_name }

    /// Get the underlying tokenizer.
    pub fn tokenizer(&self) -> &BpeTokenizer { &self.tokenizer }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt4_counter() {
        let counter = TokenCounter::gpt4();
        assert_eq!(counter.model(), "gpt-4");
        let count = counter.count("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_message_overhead() {
        let counter = TokenCounter::gpt4();
        let msg_count = counter.count_message("user", "Hello");
        let text_count = counter.count("Hello");
        // Message should be more than just text (has role + formatting overhead)
        assert!(msg_count > text_count);
    }

    #[test]
    fn test_fits() {
        let counter = TokenCounter::gpt4();
        assert!(counter.fits("short", 100));
        // Very long text shouldn't fit in 1 token
        let long_text = "a".repeat(1000);
        assert!(!counter.fits(&long_text, 1));
    }

    #[test]
    fn test_multiple_messages() {
        let counter = TokenCounter::gpt4();
        let messages = vec![
            ("system".into(), "You are helpful.".into()),
            ("user".into(), "Hello".into()),
        ];
        let count = counter.count_messages(&messages);
        assert!(count > 0);
    }
}
