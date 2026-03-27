//! Markdown structure extraction from LLM output.

use crate::parser::{OutputParser, ParseError, ParsedOutput};
use regex::Regex;
use std::collections::HashMap;

/// Extract headers, lists, and code blocks from LLM markdown output.
pub struct MarkdownOutputParser;

impl OutputParser for MarkdownOutputParser {
    fn parse(&self, raw_output: &str) -> Result<ParsedOutput, ParseError> {
        let trimmed = raw_output.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let mut map = HashMap::new();

        // Extract headers.
        let header_re = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();
        let headers: Vec<String> = header_re
            .captures_iter(trimmed)
            .map(|c| c.get(2).unwrap().as_str().to_string())
            .collect();
        if !headers.is_empty() {
            map.insert("headers".to_string(), headers.join(" | "));
        }

        // Extract list items.
        let list_re = Regex::new(r"(?m)^[\s]*[-*+]\s+(.+)$").unwrap();
        let items: Vec<String> = list_re
            .captures_iter(trimmed)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect();
        if !items.is_empty() {
            map.insert("list_items".to_string(), items.join(" | "));
        }

        // Extract code blocks.
        let code_re = Regex::new(r"```(\w*)\n([\s\S]*?)```").unwrap();
        let blocks: Vec<String> = code_re
            .captures_iter(trimmed)
            .map(|c| c.get(2).unwrap().as_str().trim().to_string())
            .collect();
        if !blocks.is_empty() {
            map.insert("code_blocks".to_string(), blocks.join(" ||| "));
        }

        if map.is_empty() {
            // No markdown structure found — return as plain text.
            return Ok(ParsedOutput::Text(trimmed.to_string()));
        }

        Ok(ParsedOutput::Structured(map))
    }
}
