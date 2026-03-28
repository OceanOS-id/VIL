//! Toxicity detection via keyword-based scoring.

use aho_corasick::AhoCorasick;

/// Keyword-based toxicity checker with configurable word lists.
pub struct ToxicityChecker {
    /// Words and their toxicity weights.
    words: Vec<(String, f32)>,
    /// Compiled Aho-Corasick automaton for fast multi-pattern matching.
    automaton: Option<AhoCorasick>,
}

impl Default for ToxicityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ToxicityChecker {
    /// Create with default (empty) word list.
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            automaton: None,
        }
    }

    /// Create with a set of default toxic keywords for demonstration.
    pub fn with_defaults() -> Self {
        let default_words = vec![
            ("hate", 0.8),
            ("kill", 0.7),
            ("violence", 0.6),
            ("abuse", 0.7),
            ("threat", 0.6),
            ("harass", 0.7),
            ("racist", 0.9),
            ("sexist", 0.9),
            ("slur", 0.8),
            ("profanity", 0.5),
        ];
        let words: Vec<(String, f32)> = default_words
            .into_iter()
            .map(|(w, s)| (w.to_string(), s))
            .collect();
        let patterns: Vec<&str> = words.iter().map(|(w, _)| w.as_str()).collect();
        let automaton = AhoCorasick::new(&patterns).ok();
        Self { words, automaton }
    }

    /// Add a word with its toxicity weight (0.0 - 1.0).
    pub fn add_word(&mut self, word: &str, weight: f32) {
        self.words
            .push((word.to_lowercase(), weight.clamp(0.0, 1.0)));
        self.rebuild_automaton();
    }

    /// Set the entire word list.
    pub fn set_words(&mut self, words: Vec<(String, f32)>) {
        self.words = words;
        self.rebuild_automaton();
    }

    fn rebuild_automaton(&mut self) {
        let patterns: Vec<&str> = self.words.iter().map(|(w, _)| w.as_str()).collect();
        self.automaton = AhoCorasick::new(&patterns).ok();
    }

    /// Score text for toxicity. Returns a score in [0.0, 1.0].
    pub fn score(&self, text: &str) -> f32 {
        let automaton = match &self.automaton {
            Some(a) => a,
            None => return 0.0,
        };

        let lower = text.to_lowercase();
        let mut total_weight = 0.0f32;
        let mut match_count = 0u32;

        for mat in automaton.find_iter(&lower) {
            let idx = mat.pattern().as_usize();
            if idx < self.words.len() {
                total_weight += self.words[idx].1;
                match_count += 1;
            }
        }

        if match_count == 0 {
            return 0.0;
        }

        // Normalize: average weight, clamped to [0, 1]
        // More matches increase the score
        let avg = total_weight / match_count as f32;
        let density =
            (match_count as f32 / (text.split_whitespace().count().max(1) as f32)).min(1.0);
        (avg * 0.7 + density * 0.3).clamp(0.0, 1.0)
    }
}
