use crate::parser::{DocMetadata, DocParser, DocSection, ParseError, ParsedDoc, SectionType};
use regex::Regex;

/// Markdown parser that extracts headings, code blocks, lists, and prose.
pub struct MarkdownParser;

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocParser for MarkdownParser {
    fn parse(&self, content: &[u8], _file_type: &str) -> Result<ParsedDoc, ParseError> {
        let text = std::str::from_utf8(content).map_err(|_| ParseError::InvalidUtf8)?;
        let sections = parse_markdown_sections(text);
        let plain_text = strip_markdown(text);
        let char_count = plain_text.len();
        let section_count = sections.len();

        Ok(ParsedDoc {
            text: plain_text,
            sections,
            metadata: DocMetadata {
                file_type: "markdown".into(),
                char_count,
                section_count,
            },
        })
    }
}

/// Strip Markdown syntax and return plain text.
fn strip_markdown(text: &str) -> String {
    let heading_re = Regex::new(r"(?m)^#{1,6}\s+").unwrap();
    let bold_re = Regex::new(r"\*\*([^*]+)\*\*").unwrap();
    let italic_re = Regex::new(r"\*([^*]+)\*").unwrap();
    let link_re = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    let img_re = Regex::new(r"!\[([^\]]*)\]\([^)]+\)").unwrap();
    let code_fence_re = Regex::new(r"(?m)^```[^\n]*\n").unwrap();
    let inline_code_re = Regex::new(r"`([^`]+)`").unwrap();

    let mut result = text.to_string();
    result = img_re.replace_all(&result, "$1").to_string();
    result = link_re.replace_all(&result, "$1").to_string();
    result = bold_re.replace_all(&result, "$1").to_string();
    result = italic_re.replace_all(&result, "$1").to_string();
    result = heading_re.replace_all(&result, "").to_string();
    result = code_fence_re.replace_all(&result, "").to_string();
    result = inline_code_re.replace_all(&result, "$1").to_string();
    result = result.replace("```", "");

    result.trim().to_string()
}

/// Parse Markdown into sections.
fn parse_markdown_sections(text: &str) -> Vec<DocSection> {
    let mut sections = Vec::new();
    let mut current_content = String::new();
    let mut current_title: Option<String> = None;
    let mut current_level: u32 = 0;
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for line in text.lines() {
        // Toggle code fences.
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block.
                sections.push(DocSection {
                    title: None,
                    content: code_block_content.trim().to_string(),
                    level: 0,
                    section_type: SectionType::Code,
                });
                code_block_content.clear();
                in_code_block = false;
            } else {
                // Start code block — flush current text.
                flush_text(
                    &mut current_content,
                    &mut current_title,
                    current_level,
                    &mut sections,
                );
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !code_block_content.is_empty() {
                code_block_content.push('\n');
            }
            code_block_content.push_str(line);
            continue;
        }

        // Detect headings.
        if let Some(caps) = detect_heading(line) {
            flush_text(
                &mut current_content,
                &mut current_title,
                current_level,
                &mut sections,
            );
            current_title = Some(caps.1.clone());
            current_level = caps.0;
            continue;
        }

        // Detect list items.
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || (trimmed.len() > 2
                && trimmed.as_bytes()[0].is_ascii_digit()
                && trimmed.contains(". "))
        {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
            continue;
        }

        // Regular text.
        if !current_content.is_empty() {
            current_content.push('\n');
        }
        current_content.push_str(line);
    }

    // Flush remaining.
    flush_text(
        &mut current_content,
        &mut current_title,
        current_level,
        &mut sections,
    );

    sections
}

fn detect_heading(line: &str) -> Option<(u32, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count() as u32;
    if level > 6 {
        return None;
    }
    let rest = trimmed[level as usize..].trim();
    if rest.is_empty() {
        return None;
    }
    Some((level, rest.to_string()))
}

fn flush_text(
    content: &mut String,
    title: &mut Option<String>,
    level: u32,
    sections: &mut Vec<DocSection>,
) {
    let trimmed = content.trim();
    if trimmed.is_empty() && title.is_none() {
        return;
    }

    if title.is_some() && trimmed.is_empty() {
        sections.push(DocSection {
            title: title.take(),
            content: String::new(),
            level,
            section_type: SectionType::Heading,
        });
    } else if title.is_some() {
        sections.push(DocSection {
            title: title.take(),
            content: trimmed.to_string(),
            level,
            section_type: SectionType::Heading,
        });
    } else if !trimmed.is_empty() {
        sections.push(DocSection {
            title: None,
            content: trimmed.to_string(),
            level: 0,
            section_type: SectionType::Text,
        });
    }

    content.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_markdown() {
        let md = "# Title\n\nSome paragraph text.\n\n## Subtitle\n\nMore text.";
        let parser = MarkdownParser::new();
        let doc = parser.parse(md.as_bytes(), "md").unwrap();
        assert!(doc.sections.len() >= 2);
        assert_eq!(doc.metadata.file_type, "markdown");
    }

    #[test]
    fn extract_headings() {
        let md = "# H1\n\nText\n\n## H2\n\nMore\n\n### H3";
        let parser = MarkdownParser::new();
        let doc = parser.parse(md.as_bytes(), "md").unwrap();
        let headings: Vec<_> = doc.sections.iter().filter(|s| s.title.is_some()).collect();
        assert!(headings.len() >= 3);
    }

    #[test]
    fn extract_code_blocks() {
        let md = "# Code\n\n```rust\nfn main() {}\n```\n\nEnd.";
        let parser = MarkdownParser::new();
        let doc = parser.parse(md.as_bytes(), "md").unwrap();
        let code_sections: Vec<_> = doc
            .sections
            .iter()
            .filter(|s| s.section_type == SectionType::Code)
            .collect();
        assert_eq!(code_sections.len(), 1);
        assert!(code_sections[0].content.contains("fn main"));
    }

    #[test]
    fn strip_markdown_formatting() {
        let md = "**bold** and *italic* and [link](http://example.com)";
        let plain = strip_markdown(md);
        assert!(plain.contains("bold"));
        assert!(plain.contains("italic"));
        assert!(plain.contains("link"));
        assert!(!plain.contains("**"));
        assert!(!plain.contains("http://"));
    }

    #[test]
    fn handles_list_items() {
        let md = "# List\n\n- item1\n- item2\n- item3";
        let parser = MarkdownParser::new();
        let doc = parser.parse(md.as_bytes(), "md").unwrap();
        assert!(doc.text.contains("item1"));
    }

    #[test]
    fn empty_input() {
        let parser = MarkdownParser::new();
        let doc = parser.parse(b"", "md").unwrap();
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn unicode_content() {
        let md = "# Titre\n\nContenu en fran\u{00e7}ais avec des accents \u{00e9}\u{00e0}\u{00fc}.";
        let parser = MarkdownParser::new();
        let doc = parser.parse(md.as_bytes(), "md").unwrap();
        assert!(doc.text.contains("fran\u{00e7}ais"));
    }
}
