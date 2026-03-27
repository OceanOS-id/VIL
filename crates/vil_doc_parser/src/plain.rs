use crate::parser::{DocMetadata, DocParser, DocSection, ParseError, ParsedDoc, SectionType};

/// Plain-text parser that splits content into paragraph-based sections.
pub struct PlainTextParser;

impl PlainTextParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlainTextParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocParser for PlainTextParser {
    fn parse(&self, content: &[u8], _file_type: &str) -> Result<ParsedDoc, ParseError> {
        let text = std::str::from_utf8(content).map_err(|_| ParseError::InvalidUtf8)?;
        let text = text.trim().to_string();

        let sections: Vec<DocSection> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| DocSection {
                title: None,
                content: p.to_string(),
                level: 0,
                section_type: SectionType::Text,
            })
            .collect();

        let char_count = text.len();
        let section_count = sections.len();

        Ok(ParsedDoc {
            text,
            sections,
            metadata: DocMetadata {
                file_type: "text".into(),
                char_count,
                section_count,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let parser = PlainTextParser::new();
        let doc = parser.parse(text.as_bytes(), "txt").unwrap();
        assert_eq!(doc.sections.len(), 3);
    }

    #[test]
    fn single_paragraph() {
        let text = "Just one paragraph with some words.";
        let parser = PlainTextParser::new();
        let doc = parser.parse(text.as_bytes(), "txt").unwrap();
        assert_eq!(doc.sections.len(), 1);
    }

    #[test]
    fn empty_input() {
        let parser = PlainTextParser::new();
        let doc = parser.parse(b"", "txt").unwrap();
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn trims_whitespace() {
        let text = "  \n\n  Some text  \n\n  ";
        let parser = PlainTextParser::new();
        let doc = parser.parse(text.as_bytes(), "txt").unwrap();
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].content, "Some text");
    }
}
