//! PII (Personally Identifiable Information) detection.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Type of PII detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PiiType {
    Email,
    Phone,
    SSN,
    CreditCard,
    IpAddress,
    Custom(String),
}

/// A detected PII match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    /// Type of PII found.
    pub pii_type: PiiType,
    /// The matched value.
    pub value: String,
    /// Byte position in the original text.
    pub position: usize,
}

/// Regex-based PII detector.
pub struct PiiDetector {
    email_re: Regex,
    phone_re: Regex,
    ssn_re: Regex,
    credit_card_re: Regex,
    ip_re: Regex,
    custom_patterns: Vec<(String, Regex)>,
}

impl Default for PiiDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiDetector {
    /// Create a new PII detector with built-in patterns.
    pub fn new() -> Self {
        Self {
            email_re: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            phone_re: Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b")
                .unwrap(),
            ssn_re: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            credit_card_re: Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap(),
            ip_re: Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b",
            )
            .unwrap(),
            custom_patterns: Vec::new(),
        }
    }

    /// Add a custom PII pattern.
    pub fn add_custom_pattern(&mut self, name: &str, pattern: &str) -> Result<(), regex::Error> {
        let re = Regex::new(pattern)?;
        self.custom_patterns.push((name.to_string(), re));
        Ok(())
    }

    /// Detect all PII in the given text.
    pub fn detect(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for m in self.email_re.find_iter(text) {
            matches.push(PiiMatch {
                pii_type: PiiType::Email,
                value: m.as_str().to_string(),
                position: m.start(),
            });
        }

        for m in self.phone_re.find_iter(text) {
            matches.push(PiiMatch {
                pii_type: PiiType::Phone,
                value: m.as_str().to_string(),
                position: m.start(),
            });
        }

        // SSN detection — exclude matches that are also credit card substrings
        for m in self.ssn_re.find_iter(text) {
            // Check this isn't part of a credit card number
            let val = m.as_str();
            if !self
                .credit_card_re
                .is_match(&text[m.start().saturating_sub(10)..text.len().min(m.end() + 10)])
            {
                matches.push(PiiMatch {
                    pii_type: PiiType::SSN,
                    value: val.to_string(),
                    position: m.start(),
                });
            }
        }

        for m in self.credit_card_re.find_iter(text) {
            matches.push(PiiMatch {
                pii_type: PiiType::CreditCard,
                value: m.as_str().to_string(),
                position: m.start(),
            });
        }

        for m in self.ip_re.find_iter(text) {
            matches.push(PiiMatch {
                pii_type: PiiType::IpAddress,
                value: m.as_str().to_string(),
                position: m.start(),
            });
        }

        for (name, re) in &self.custom_patterns {
            for m in re.find_iter(text) {
                matches.push(PiiMatch {
                    pii_type: PiiType::Custom(name.clone()),
                    value: m.as_str().to_string(),
                    position: m.start(),
                });
            }
        }

        // Sort by position
        matches.sort_by_key(|m| m.position);
        matches
    }
}
