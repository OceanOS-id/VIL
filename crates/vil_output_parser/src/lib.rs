//! # vil_output_parser
//!
//! N10 — LLM Output Parser: extract JSON, regex patterns, and markdown structures
//! from raw LLM output. Includes JSON repair for common LLM mistakes.

pub mod json_parser;
pub mod markdown_parser;
pub mod parser;
pub mod regex_parser;

pub use json_parser::{repair_json, JsonOutputParser};
pub use markdown_parser::MarkdownOutputParser;
pub use parser::{OutputParser, ParseError, ParsedOutput};
pub use regex_parser::RegexOutputParser;

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::OutputParserPlugin;
pub use vil_semantic::{ParseEvent, ParseFault, ParserState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_json() {
        let parser = JsonOutputParser;
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        match result {
            ParsedOutput::Json(v) => assert_eq!(v["key"], "value"),
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn test_json_in_markdown_fences() {
        let parser = JsonOutputParser;
        let input = "Here is the result:\n```json\n{\"name\": \"Alice\"}\n```\nDone.";
        let result = parser.parse(input).unwrap();
        match result {
            ParsedOutput::Json(v) => assert_eq!(v["name"], "Alice"),
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn test_json_with_trailing_comma() {
        let result = repair_json(r#"{"a": 1, "b": 2,}"#).unwrap();
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"], 2);
    }

    #[test]
    fn test_json_with_single_quotes() {
        let result = repair_json("{'key': 'value'}").unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_json_with_unquoted_keys() {
        let result = repair_json("{name: \"Bob\"}").unwrap();
        assert_eq!(result["name"], "Bob");
    }

    #[test]
    fn test_json_embedded_in_text() {
        let parser = JsonOutputParser;
        let input = "Sure! Here is the JSON:\n{\"answer\": 42}\nHope that helps!";
        let result = parser.parse(input).unwrap();
        match result {
            ParsedOutput::Json(v) => assert_eq!(v["answer"], 42),
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn test_regex_extraction() {
        let parser = RegexOutputParser::new(r"Name: (?P<name>\w+), Age: (?P<age>\d+)").unwrap();
        let result = parser.parse("Name: Alice, Age: 30").unwrap();
        match result {
            ParsedOutput::Structured(map) => {
                assert_eq!(map["name"], "Alice");
                assert_eq!(map["age"], "30");
            }
            _ => panic!("expected Structured"),
        }
    }

    #[test]
    fn test_markdown_parsing_headers() {
        let parser = MarkdownOutputParser;
        let input = "# Title\n\nSome text\n\n## Section\n\nMore text";
        let result = parser.parse(input).unwrap();
        match result {
            ParsedOutput::Structured(map) => {
                assert!(map["headers"].contains("Title"));
                assert!(map["headers"].contains("Section"));
            }
            _ => panic!("expected Structured"),
        }
    }

    #[test]
    fn test_markdown_parsing_lists() {
        let parser = MarkdownOutputParser;
        let input = "- item one\n- item two\n- item three";
        let result = parser.parse(input).unwrap();
        match result {
            ParsedOutput::Structured(map) => {
                assert!(map["list_items"].contains("item one"));
                assert!(map["list_items"].contains("item three"));
            }
            _ => panic!("expected Structured"),
        }
    }

    #[test]
    fn test_no_match_error() {
        let parser = RegexOutputParser::new(r"XYZ_(?P<id>\d+)").unwrap();
        let result = parser.parse("no match here");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_input() {
        let parser = JsonOutputParser;
        let result = parser.parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_plain_text_markdown() {
        let parser = MarkdownOutputParser;
        let result = parser.parse("Just plain text, no markdown.").unwrap();
        match result {
            ParsedOutput::Text(t) => assert_eq!(t, "Just plain text, no markdown."),
            _ => panic!("expected Text"),
        }
    }
}
