/// Configuration for speculative decoding.
#[derive(Debug, Clone)]
pub struct SpeculativeConfig {
    /// Maximum number of draft tokens to generate per iteration.
    pub max_draft_tokens: usize,
    /// Maximum total tokens for the entire generation.
    pub max_total_tokens: usize,
    /// Maximum decoding iterations (safety limit).
    pub max_iterations: usize,
}

impl Default for SpeculativeConfig {
    fn default() -> Self {
        Self {
            max_draft_tokens: 5,
            max_total_tokens: 256,
            max_iterations: 100,
        }
    }
}

impl SpeculativeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_draft_tokens(mut self, n: usize) -> Self {
        self.max_draft_tokens = n;
        self
    }

    pub fn max_total_tokens(mut self, n: usize) -> Self {
        self.max_total_tokens = n;
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }
}
