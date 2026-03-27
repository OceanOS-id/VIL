// ── N01: Quality Filter ──────────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// Configuration for quality-based filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFilter {
    pub min_length: usize,
    pub max_length: usize,
    pub min_quality_score: f64,
    pub language: Option<String>,
}

impl Default for QualityFilter {
    fn default() -> Self {
        Self {
            min_length: 10,
            max_length: 100_000,
            min_quality_score: 0.3,
            language: None,
        }
    }
}

impl QualityFilter {
    pub fn new(min_length: usize, max_length: usize, min_quality_score: f64) -> Self {
        Self {
            min_length,
            max_length,
            min_quality_score,
            language: None,
        }
    }

    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    /// Returns true if the text passes all quality checks.
    pub fn passes(&self, text: &str) -> bool {
        let len = text.len();
        if len < self.min_length || len > self.max_length {
            return false;
        }
        let score = compute_quality_score(text);
        score >= self.min_quality_score
    }

    /// Apply filter to a batch, returning only passing texts.
    pub fn filter_batch(&self, texts: &[String]) -> Vec<String> {
        texts.iter().filter(|t| self.passes(t)).cloned().collect()
    }
}

/// Heuristic quality score (0.0–1.0).
/// Considers: non-whitespace ratio, alphanumeric ratio, sentence structure.
pub fn compute_quality_score(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let chars: Vec<char> = text.chars().collect();
    let total = chars.len() as f64;

    // Non-whitespace ratio
    let non_ws = chars.iter().filter(|c| !c.is_whitespace()).count() as f64;
    let ws_ratio = non_ws / total;

    // Alphanumeric ratio
    let alnum = chars.iter().filter(|c| c.is_alphanumeric()).count() as f64;
    let alnum_ratio = alnum / total;

    // Has sentence-like structure (starts uppercase, has punctuation)
    let has_structure = if text.trim().starts_with(|c: char| c.is_uppercase())
        && text.contains(|c: char| ".!?".contains(c))
    {
        1.0
    } else {
        0.5
    };

    (ws_ratio * 0.3 + alnum_ratio * 0.4 + has_structure * 0.3).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_passes_good_text() {
        let f = QualityFilter::new(5, 1000, 0.3);
        assert!(f.passes("This is a well-formed sentence with good content."));
    }

    #[test]
    fn filter_rejects_too_short() {
        let f = QualityFilter::new(100, 1000, 0.0);
        assert!(!f.passes("short"));
    }

    #[test]
    fn filter_rejects_too_long() {
        let f = QualityFilter::new(1, 10, 0.0);
        assert!(!f.passes("this is way too long for the filter"));
    }

    #[test]
    fn filter_batch_works() {
        let f = QualityFilter::new(5, 100, 0.0);
        let input = vec!["ok text here".into(), "no".into(), "another good one".into()];
        let result = f.filter_batch(&input);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn quality_score_empty() {
        assert_eq!(compute_quality_score(""), 0.0);
    }
}
