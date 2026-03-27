use crate::patterns::PatternEntry;
use crate::result::RiskLevel;

/// Configuration for the PromptShield.
#[derive(Clone)]
pub struct ShieldConfig {
    /// Minimum risk level to block (default: High)
    pub block_threshold: RiskLevel,
    /// Enable heuristic scoring in addition to patterns
    pub enable_heuristics: bool,
    /// Custom patterns to add
    pub custom_patterns: Vec<PatternEntry>,
    /// Patterns to exclude by ID
    pub excluded_pattern_ids: Vec<String>,
    /// Allow list: text containing these strings is always safe
    pub allow_list: Vec<String>,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            block_threshold: RiskLevel::High,
            enable_heuristics: true,
            custom_patterns: Vec::new(),
            excluded_pattern_ids: Vec::new(),
            allow_list: Vec::new(),
        }
    }
}

impl ShieldConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block_threshold(mut self, level: RiskLevel) -> Self {
        self.block_threshold = level;
        self
    }

    pub fn heuristics(mut self, enable: bool) -> Self {
        self.enable_heuristics = enable;
        self
    }

    pub fn add_pattern(mut self, entry: PatternEntry) -> Self {
        self.custom_patterns.push(entry);
        self
    }

    pub fn exclude_pattern(mut self, id: impl Into<String>) -> Self {
        self.excluded_pattern_ids.push(id.into());
        self
    }

    pub fn allow(mut self, text: impl Into<String>) -> Self {
        self.allow_list.push(text.into());
        self
    }
}
