use crate::parser::{DocMetadata, DocParser, DocSection, ParseError, ParsedDoc, SectionType};

/// CSV parser that converts rows into readable structured text.
///
/// Each data row becomes a section whose content maps column headers to values.
/// Example: `"name: Alice, age: 30, city: NYC"`.
pub struct CsvParser;

impl CsvParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CsvParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocParser for CsvParser {
    fn parse(&self, content: &[u8], _file_type: &str) -> Result<ParsedDoc, ParseError> {
        let text = std::str::from_utf8(content).map_err(|_| ParseError::InvalidUtf8)?;

        let mut lines = text.lines();
        let header_line = match lines.next() {
            Some(h) if !h.trim().is_empty() => h,
            _ => {
                return Ok(ParsedDoc {
                    text: String::new(),
                    sections: Vec::new(),
                    metadata: DocMetadata::default(),
                });
            }
        };

        let headers: Vec<&str> = header_line.split(',').map(|h| h.trim()).collect();
        let mut sections = Vec::new();
        let mut full_text = String::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let values: Vec<&str> = trimmed.split(',').map(|v| v.trim()).collect();
            let pairs: Vec<String> = headers
                .iter()
                .zip(values.iter())
                .map(|(h, v)| format!("{h}: {v}"))
                .collect();
            let readable = pairs.join(", ");

            if !full_text.is_empty() {
                full_text.push('\n');
            }
            full_text.push_str(&readable);

            sections.push(DocSection {
                title: None,
                content: readable,
                level: 0,
                section_type: SectionType::Table,
            });
        }

        let char_count = full_text.len();
        let section_count = sections.len();

        Ok(ParsedDoc {
            text: full_text,
            sections,
            metadata: DocMetadata {
                file_type: "csv".into(),
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
    fn basic_csv_parsing() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let parser = CsvParser::new();
        let doc = parser.parse(csv.as_bytes(), "csv").unwrap();
        assert_eq!(doc.sections.len(), 2);
        assert!(doc.sections[0].content.contains("name: Alice"));
        assert!(doc.sections[0].content.contains("age: 30"));
    }

    #[test]
    fn header_only() {
        let csv = "col1,col2,col3";
        let parser = CsvParser::new();
        let doc = parser.parse(csv.as_bytes(), "csv").unwrap();
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn empty_input() {
        let parser = CsvParser::new();
        let doc = parser.parse(b"", "csv").unwrap();
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn skips_empty_rows() {
        let csv = "a,b\n1,2\n\n3,4";
        let parser = CsvParser::new();
        let doc = parser.parse(csv.as_bytes(), "csv").unwrap();
        assert_eq!(doc.sections.len(), 2);
    }

    #[test]
    fn section_type_is_table() {
        let csv = "x,y\n1,2";
        let parser = CsvParser::new();
        let doc = parser.parse(csv.as_bytes(), "csv").unwrap();
        assert_eq!(doc.sections[0].section_type, SectionType::Table);
    }
}
