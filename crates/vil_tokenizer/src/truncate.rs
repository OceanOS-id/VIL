use crate::bpe::BpeTokenizer;

/// Strategy for truncating text to fit token limits.
pub enum TruncateStrategy {
    /// Cut from the end, preserve beginning
    TailDrop,
    /// Cut from the beginning, preserve end
    HeadDrop,
    /// Keep beginning and end, cut middle (insert "..." marker)
    MiddleDrop,
}

/// Truncate text to fit within max_tokens.
///
/// Returns the truncated text. If text already fits, returns it unchanged.
pub fn truncate_to_tokens(
    tokenizer: &BpeTokenizer,
    text: &str,
    max_tokens: usize,
    strategy: TruncateStrategy,
) -> String {
    let current = tokenizer.count_tokens(text);
    if current <= max_tokens {
        return text.to_string();
    }

    // Estimate target character count from token budget using the ratio
    let ratio = tokenizer.count_tokens("aaaa") as f64 / 4.0;
    let target_chars = if ratio > 0.0 {
        (max_tokens as f64 / ratio * 4.0) as usize
    } else {
        max_tokens * 4
    };
    let target_chars = target_chars.min(text.len());

    match strategy {
        TruncateStrategy::TailDrop => {
            // Binary search for the right cut point
            let mut end = target_chars;
            while end > 0 && tokenizer.count_tokens(&text[..text.floor_char_boundary(end)]) > max_tokens {
                end = end.saturating_sub(end / 10 + 1);
            }
            // Adjust to char boundary
            let end = text.floor_char_boundary(end);
            text[..end].to_string()
        }
        TruncateStrategy::HeadDrop => {
            let start = text.len().saturating_sub(target_chars);
            let start = text.ceil_char_boundary(start);
            text[start..].to_string()
        }
        TruncateStrategy::MiddleDrop => {
            let half = target_chars / 2;
            let start_end = text.floor_char_boundary(half);
            let end_start = text.ceil_char_boundary(text.len().saturating_sub(half));
            format!("{}...{}", &text[..start_end], &text[end_start..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{Vocabulary, VocabSource, BuiltInVocab};

    fn make_tokenizer() -> BpeTokenizer {
        let vocab = Vocabulary::load(VocabSource::BuiltIn(BuiltInVocab::Cl100kBase)).unwrap();
        BpeTokenizer::new(vocab)
    }

    #[test]
    fn test_no_truncation_needed() {
        let tok = make_tokenizer();
        let result = truncate_to_tokens(&tok, "short", 100, TruncateStrategy::TailDrop);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_tail_drop() {
        let tok = make_tokenizer();
        let long = "a".repeat(1000);
        let result = truncate_to_tokens(&tok, &long, 10, TruncateStrategy::TailDrop);
        assert!(tok.count_tokens(&result) <= 10, "got {} tokens", tok.count_tokens(&result));
        assert!(result.len() < long.len());
    }

    #[test]
    fn test_middle_drop() {
        let tok = make_tokenizer();
        let long = "a".repeat(1000);
        let result = truncate_to_tokens(&tok, &long, 10, TruncateStrategy::MiddleDrop);
        assert!(result.contains("..."));
    }
}
