use crate::parser::{DocMetadata, DocParser, DocSection, ParseError, ParsedDoc, SectionType};
use regex::Regex;

/// Simple HTML-to-text parser.
///
/// Strips tags, decodes common entities, and extracts text content.
/// Block-level elements (`<p>`, `<div>`, `<br>`, headings) produce newlines.
pub struct HtmlParser;

impl HtmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocParser for HtmlParser {
    fn parse(&self, content: &[u8], _file_type: &str) -> Result<ParsedDoc, ParseError> {
        let html = std::str::from_utf8(content).map_err(|_| ParseError::InvalidUtf8)?;
        let (text, sections) = strip_html(html);
        let char_count = text.len();
        let section_count = sections.len();

        Ok(ParsedDoc {
            text,
            sections,
            metadata: DocMetadata {
                file_type: "html".into(),
                char_count,
                section_count,
            },
        })
    }
}

fn strip_html(html: &str) -> (String, Vec<DocSection>) {
    let mut sections = Vec::new();

    // Remove script and style blocks entirely.
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let cleaned = script_re.replace_all(html, "");
    let cleaned = style_re.replace_all(&cleaned, "");

    // Extract headings.
    let heading_re = Regex::new(r"(?is)<h(\d)[^>]*>(.*?)</h\d>").unwrap();
    for cap in heading_re.captures_iter(&cleaned) {
        let level: u32 = cap[1].parse().unwrap_or(1);
        let inner = strip_tags(&cap[2]);
        let trimmed = inner.trim().to_string();
        if !trimmed.is_empty() {
            sections.push(DocSection {
                title: Some(trimmed),
                content: String::new(),
                level,
                section_type: SectionType::Heading,
            });
        }
    }

    // Insert newlines before block elements.
    let block_re = Regex::new(r"(?i)<(/?)(p|div|br|h\d|li|tr|blockquote|pre)[^>]*>").unwrap();
    let with_newlines = block_re.replace_all(&cleaned, "\n");

    // Strip remaining tags.
    let text = strip_tags(&with_newlines);

    // Decode common HTML entities.
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse multiple newlines.
    let collapse_re = Regex::new(r"\n{3,}").unwrap();
    let text = collapse_re.replace_all(&text, "\n\n").trim().to_string();

    // Build text sections from paragraphs.
    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if !trimmed.is_empty() && !sections.iter().any(|s| s.title.as_deref() == Some(trimmed)) {
            sections.push(DocSection {
                title: None,
                content: trimmed.to_string(),
                level: 0,
                section_type: SectionType::Text,
            });
        }
    }

    (text, sections)
}

fn strip_tags(html: &str) -> String {
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    tag_re.replace_all(html, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_basic_html() {
        let html = "<p>Hello <b>world</b></p>";
        let parser = HtmlParser::new();
        let doc = parser.parse(html.as_bytes(), "html").unwrap();
        assert!(doc.text.contains("Hello"));
        assert!(doc.text.contains("world"));
        assert!(!doc.text.contains("<b>"));
    }

    #[test]
    fn extracts_headings() {
        let html = "<h1>Title</h1><p>Body</p><h2>Sub</h2>";
        let parser = HtmlParser::new();
        let doc = parser.parse(html.as_bytes(), "html").unwrap();
        let headings: Vec<_> = doc.sections.iter().filter(|s| s.title.is_some()).collect();
        assert!(headings.len() >= 2);
    }

    #[test]
    fn removes_script_and_style() {
        let html = "<p>Hello</p><script>alert('x')</script><style>.x{}</style><p>End</p>";
        let parser = HtmlParser::new();
        let doc = parser.parse(html.as_bytes(), "html").unwrap();
        assert!(!doc.text.contains("alert"));
        assert!(!doc.text.contains(".x{"));
        assert!(doc.text.contains("Hello"));
    }

    #[test]
    fn decodes_entities() {
        let html = "<p>A &amp; B &lt; C &gt; D</p>";
        let parser = HtmlParser::new();
        let doc = parser.parse(html.as_bytes(), "html").unwrap();
        assert!(doc.text.contains("A & B < C > D"));
    }

    #[test]
    fn empty_input() {
        let parser = HtmlParser::new();
        let doc = parser.parse(b"", "html").unwrap();
        assert!(doc.text.is_empty());
    }
}
