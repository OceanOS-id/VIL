//! JSON extraction from LLM output.

use crate::parser::{OutputParser, ParseError, ParsedOutput};
use regex::Regex;

/// Extract JSON from LLM output, handling markdown fences and trailing text.
pub struct JsonOutputParser;

impl OutputParser for JsonOutputParser {
    fn parse(&self, raw_output: &str) -> Result<ParsedOutput, ParseError> {
        let trimmed = raw_output.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // Try 1: direct parse.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return Ok(ParsedOutput::Json(v));
        }

        // Try 2: extract from markdown code fences.
        let fence_re = Regex::new(r"```(?:json)?\s*\n?([\s\S]*?)\n?\s*```").unwrap();
        if let Some(caps) = fence_re.captures(trimmed) {
            let json_str = caps.get(1).unwrap().as_str().trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                return Ok(ParsedOutput::Json(v));
            }
            // Try repairing the fenced content.
            if let Ok(v) = repair_json(json_str) {
                return Ok(ParsedOutput::Json(v));
            }
        }

        // Try 3: find first { ... } or [ ... ] block.
        if let Some(json_str) = extract_json_block(trimmed) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                return Ok(ParsedOutput::Json(v));
            }
            if let Ok(v) = repair_json(&json_str) {
                return Ok(ParsedOutput::Json(v));
            }
        }

        // Try 4: repair the whole thing.
        if let Ok(v) = repair_json(trimmed) {
            return Ok(ParsedOutput::Json(v));
        }

        Err(ParseError::InvalidJson("could not extract valid JSON".into()))
    }
}

/// Extract the outermost JSON object or array from text.
fn extract_json_block(text: &str) -> Option<String> {
    let (open, close) = if let Some(pos) = text.find('{') {
        if let Some(arr_pos) = text.find('[') {
            if arr_pos < pos { ('[', ']') } else { ('{', '}') }
        } else {
            ('{', '}')
        }
    } else if text.find('[').is_some() {
        ('[', ']')
    } else {
        return None;
    };

    let start = text.find(open)?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(text[start..start + i + 1].to_string());
            }
        }
    }

    None
}

/// Attempt to repair common LLM JSON mistakes.
pub fn repair_json(broken: &str) -> Result<serde_json::Value, ParseError> {
    let mut s = broken.trim().to_string();

    // Fix single quotes -> double quotes (only outside existing double-quoted strings).
    s = fix_single_quotes(&s);

    // Remove trailing commas before } or ].
    let trailing_re = Regex::new(r",\s*([}\]])").unwrap();
    s = trailing_re.replace_all(&s, "$1").to_string();

    // Try to add missing quotes to unquoted keys: word: -> "word":
    let unquoted_key_re = Regex::new(r"(?m)(\{|,)\s*([a-zA-Z_]\w*)\s*:").unwrap();
    s = unquoted_key_re.replace_all(&s, r#"$1 "$2":"#).to_string();

    serde_json::from_str(&s).map_err(|e| ParseError::InvalidJson(e.to_string()))
}

/// Naive single-quote to double-quote fixer.
fn fix_single_quotes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_double = false;
    let mut prev = '\0';

    for ch in s.chars() {
        if ch == '"' && prev != '\\' {
            in_double = !in_double;
            result.push(ch);
        } else if ch == '\'' && !in_double {
            result.push('"');
        } else {
            result.push(ch);
        }
        prev = ch;
    }

    result
}
