use serde::{Deserialize, Serialize};
use vil_macros::VilAiEvent;

/// Risk level of detected injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// A single detected threat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub pattern_id: String,
    pub category: ThreatCategory,
    pub risk: RiskLevel,
    pub matched_text: String,
    pub position: usize,
    pub description: String,
}

/// Categories of prompt injection threats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatCategory {
    /// "Ignore previous instructions" type attacks
    InstructionOverride,
    /// Attempts to extract system prompt
    SystemPromptLeak,
    /// Role-playing to bypass safety
    RolePlayJailbreak,
    /// Encoding/obfuscation to bypass filters
    EncodingBypass,
    /// Attempts to execute code or access system
    CodeInjection,
    /// Social engineering / manipulation
    SocialEngineering,
    /// Data exfiltration attempts
    DataExfiltration,
    /// Custom pattern match
    Custom(String),
}

/// Result of a prompt shield scan.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct ScanResult {
    pub safe: bool,
    pub risk_level: RiskLevel,
    pub threats: Vec<Threat>,
    pub score: f64, // 0.0 (safe) to 1.0 (dangerous)
    pub scan_time_us: u64,
}

impl ScanResult {
    pub fn safe() -> Self {
        Self {
            safe: true,
            risk_level: RiskLevel::None,
            threats: vec![],
            score: 0.0,
            scan_time_us: 0,
        }
    }

    pub fn is_blocked(&self, threshold: RiskLevel) -> bool {
        self.risk_level >= threshold
    }
}
