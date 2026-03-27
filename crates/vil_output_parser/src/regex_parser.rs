//! Regex-based extraction from LLM output.

use crate::parser::{OutputParser, ParseError, ParsedOutput};
use regex::Regex;
use std::collections::HashMap;

/// Extract data via named capture groups.
pub struct RegexOutputParser {
    pattern: Regex,
}

impl RegexOutputParser {
    pub fn new(pattern: &str) -> Result<Self, ParseError> {
        Regex::new(pattern)
            .map(|r| Self { pattern: r })
            .map_err(|e| ParseError::InvalidRegex(e.to_string()))
    }
}

impl OutputParser for RegexOutputParser {
    fn parse(&self, raw_output: &str) -> Result<ParsedOutput, ParseError> {
        let trimmed = raw_output.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        if let Some(caps) = self.pattern.captures(trimmed) {
            let mut map = HashMap::new();
            for name in self.pattern.capture_names().flatten() {
                if let Some(m) = caps.name(name) {
                    map.insert(name.to_string(), m.as_str().to_string());
                }
            }
            if map.is_empty() {
                // No named groups — return full match as text.
                if let Some(m) = caps.get(0) {
                    return Ok(ParsedOutput::Text(m.as_str().to_string()));
                }
            }
            Ok(ParsedOutput::Structured(map))
        } else {
            Err(ParseError::NoMatch("regex did not match".into()))
        }
    }
}
