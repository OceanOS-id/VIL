//! PII redaction before indexing.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A pattern to redact from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactPattern {
    pub name: String,
    pub regex: String,
    pub replacement: String,
}

/// Redactor that strips PII from text before indexing.
pub struct Redactor {
    patterns: Vec<(RedactPattern, Regex)>,
}

impl Redactor {
    pub fn new(patterns: Vec<RedactPattern>) -> Self {
        let compiled: Vec<(RedactPattern, Regex)> = patterns
            .into_iter()
            .filter_map(|p| Regex::new(&p.regex).ok().map(|r| (p, r)))
            .collect();
        Self { patterns: compiled }
    }

    /// Redact all matching patterns, returning the cleaned text and a list of what was redacted.
    pub fn redact(&self, text: &str) -> RedactResult {
        let mut output = text.to_string();
        let mut redactions = Vec::new();

        for (pat, re) in &self.patterns {
            let matches: Vec<String> = re
                .find_iter(&output)
                .map(|m| m.as_str().to_string())
                .collect();
            if !matches.is_empty() {
                for m in &matches {
                    redactions.push(Redaction {
                        pattern_name: pat.name.clone(),
                        original_length: m.len(),
                    });
                }
                output = re
                    .replace_all(&output, pat.replacement.as_str())
                    .to_string();
            }
        }

        RedactResult {
            text: output,
            redactions,
        }
    }
}

/// Result of redaction.
#[derive(Debug, Clone)]
pub struct RedactResult {
    pub text: String,
    pub redactions: Vec<Redaction>,
}

/// Info about a single redaction.
#[derive(Debug, Clone)]
pub struct Redaction {
    pub pattern_name: String,
    pub original_length: usize,
}

/// Pre-built patterns for common PII types.
pub fn default_pii_patterns() -> Vec<RedactPattern> {
    // Order matters: longer/more-specific patterns first to avoid partial matches.
    vec![
        RedactPattern {
            name: "email".into(),
            regex: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".into(),
            replacement: "[REDACTED_EMAIL]".into(),
        },
        RedactPattern {
            name: "credit_card".into(),
            regex: r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".into(),
            replacement: "[REDACTED_CC]".into(),
        },
        RedactPattern {
            name: "ssn".into(),
            regex: r"\b\d{3}-\d{2}-\d{4}\b".into(),
            replacement: "[REDACTED_SSN]".into(),
        },
        RedactPattern {
            name: "phone".into(),
            regex: r"\b(\+?1[-.\s]?)?(\(?\d{3}\)?[-.\s]?)?\d{3}[-.\s]?\d{4}\b".into(),
            replacement: "[REDACTED_PHONE]".into(),
        },
    ]
}
