//! VIL Native Document Parser (H02)
//!
//! Zero-dependency document parsing for RAG pipelines.
//! Converts Markdown, HTML, plain text, and CSV into structured `ParsedDoc`
//! representations ready for chunking and embedding.
//!
//! ## Parsers
//!
//! | Parser | Input | Description |
//! |---|---|---|
//! | [`MarkdownParser`] | `.md` | Strips syntax, extracts headings, code blocks, lists |
//! | [`HtmlParser`] | `.html` | Strips tags, extracts text, respects block elements |
//! | [`PlainTextParser`] | `.txt` | Paragraph splitting |
//! | [`CsvParser`] | `.csv` | Converts rows to readable key-value text |
//!
//! All parsers implement the [`DocParser`] trait.

pub mod csv_parser;
pub mod html;
pub mod markdown;
pub mod parser;
pub mod plain;

pub use csv_parser::CsvParser;
pub use html::HtmlParser;
pub use markdown::MarkdownParser;
pub use parser::{DocMetadata, DocParser, DocSection, ParseError, ParsedDoc, SectionType};
pub use plain::PlainTextParser;

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::DocParserPlugin;
pub use semantic::{DocParserState, ParseEvent, ParseFault};

/// Auto-detect parser based on file extension and parse the content.
pub fn auto_parse(content: &[u8], file_type: &str) -> Result<ParsedDoc, ParseError> {
    match file_type.to_lowercase().as_str() {
        "md" | "markdown" => MarkdownParser::new().parse(content, file_type),
        "html" | "htm" => HtmlParser::new().parse(content, file_type),
        "csv" | "tsv" => CsvParser::new().parse(content, file_type),
        "txt" | "text" | "" => PlainTextParser::new().parse(content, file_type),
        other => Err(ParseError::UnsupportedFormat(other.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_parse_markdown() {
        let doc = auto_parse(b"# Hello\n\nWorld", "md").unwrap();
        assert_eq!(doc.metadata.file_type, "markdown");
    }

    #[test]
    fn auto_parse_html() {
        let doc = auto_parse(b"<p>Hello</p>", "html").unwrap();
        assert_eq!(doc.metadata.file_type, "html");
    }

    #[test]
    fn auto_parse_csv() {
        let doc = auto_parse(b"a,b\n1,2", "csv").unwrap();
        assert_eq!(doc.metadata.file_type, "csv");
    }

    #[test]
    fn auto_parse_txt() {
        let doc = auto_parse(b"Hello world", "txt").unwrap();
        assert_eq!(doc.metadata.file_type, "text");
    }

    #[test]
    fn auto_parse_unsupported() {
        let err = auto_parse(b"data", "pdf").unwrap_err();
        assert!(format!("{err}").contains("pdf"));
    }
}
