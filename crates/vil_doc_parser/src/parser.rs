use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of content a section contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    Text,
    Heading,
    Code,
    Table,
    List,
    Image(String), // caption
}

/// A single section extracted from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSection {
    /// Optional title (e.g. heading text).
    pub title: Option<String>,
    /// Section content.
    pub content: String,
    /// Heading level (1-6 for Markdown/HTML, 0 for non-heading).
    pub level: u32,
    /// What kind of section this is.
    pub section_type: SectionType,
}

/// Metadata extracted from a document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocMetadata {
    /// Detected or declared file type (e.g. "markdown", "html", "csv").
    pub file_type: String,
    /// Total character count of extracted text.
    pub char_count: usize,
    /// Number of sections.
    pub section_count: usize,
}

/// A fully parsed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDoc {
    /// Plain-text representation of the entire document.
    pub text: String,
    /// Structured sections.
    pub sections: Vec<DocSection>,
    /// Document-level metadata.
    pub metadata: DocMetadata,
}

/// Errors that may occur during parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// The file type is not supported by this parser.
    UnsupportedFormat(String),
    /// The input bytes are not valid UTF-8.
    InvalidUtf8,
    /// Generic parse failure.
    ParseFailed(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnsupportedFormat(ft) => write!(f, "unsupported format: {ft}"),
            ParseError::InvalidUtf8 => write!(f, "invalid UTF-8"),
            ParseError::ParseFailed(msg) => write!(f, "parse failed: {msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Trait for document parsers.
pub trait DocParser: Send + Sync {
    /// Parse raw bytes into a structured `ParsedDoc`.
    ///
    /// `file_type` is a hint (e.g. "md", "html", "csv", "txt").
    fn parse(&self, content: &[u8], file_type: &str) -> Result<ParsedDoc, ParseError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let e = ParseError::UnsupportedFormat("pdf".into());
        assert!(format!("{e}").contains("pdf"));

        let e = ParseError::InvalidUtf8;
        assert!(format!("{e}").contains("UTF-8"));
    }

    #[test]
    fn section_type_serde_roundtrip() {
        let st = SectionType::Image("photo".into());
        let json = serde_json::to_string(&st).unwrap();
        let back: SectionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SectionType::Image("photo".into()));
    }

    #[test]
    fn parsed_doc_serde() {
        let doc = ParsedDoc {
            text: "hello".into(),
            sections: vec![],
            metadata: DocMetadata {
                file_type: "txt".into(),
                char_count: 5,
                section_count: 0,
            },
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back: ParsedDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text, "hello");
    }
}
