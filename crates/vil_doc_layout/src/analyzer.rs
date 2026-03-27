//! LayoutAnalyzer — rule-based layout detection from text.

use crate::element::{DocSection, LayoutElement, LayoutRegion};
use crate::rules::LayoutRules;

/// Analyzes document text and produces layout regions and sections.
pub struct LayoutAnalyzer {
    rules: LayoutRules,
}

impl Default for LayoutAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutAnalyzer {
    /// Create a new analyzer with default rules.
    pub fn new() -> Self {
        Self {
            rules: LayoutRules::new(),
        }
    }

    /// Analyze text and return a list of layout regions.
    pub fn analyze(&self, text: &str) -> Vec<LayoutRegion> {
        if text.is_empty() {
            return Vec::new();
        }

        let lines: Vec<&str> = text.lines().collect();
        let mut regions: Vec<LayoutRegion> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim_end();

            // Skip empty lines
            if line.trim().is_empty() {
                i += 1;
                continue;
            }

            // Horizontal rule
            if self.rules.is_horizontal_rule(line.trim()) {
                regions.push(LayoutRegion {
                    element: LayoutElement::HorizontalRule,
                    text: line.to_string(),
                    start_line: i,
                    end_line: i,
                    confidence: 1.0,
                });
                i += 1;
                continue;
            }

            // Heading
            if let Some((level, heading_text)) = self.rules.match_heading(line) {
                regions.push(LayoutRegion {
                    element: LayoutElement::Heading(level),
                    text: heading_text.to_string(),
                    start_line: i,
                    end_line: i,
                    confidence: 1.0,
                });
                i += 1;
                continue;
            }

            // Code block
            if let Some(lang_tag) = self.rules.match_code_fence(line) {
                let lang = if lang_tag.is_empty() {
                    None
                } else {
                    Some(lang_tag.to_string())
                };
                let start = i;
                let mut code_lines = Vec::new();
                i += 1;
                while i < lines.len() {
                    if lines[i].trim_end().starts_with("```") {
                        break;
                    }
                    code_lines.push(lines[i]);
                    i += 1;
                }
                let end = i.min(lines.len() - 1);
                regions.push(LayoutRegion {
                    element: LayoutElement::CodeBlock(lang),
                    text: code_lines.join("\n"),
                    start_line: start,
                    end_line: end,
                    confidence: 1.0,
                });
                i += 1; // skip closing fence
                continue;
            }

            // Image
            if let Some(caption) = self.rules.match_image(line.trim()) {
                regions.push(LayoutRegion {
                    element: LayoutElement::Image(caption.map(|s| s.to_string())),
                    text: line.to_string(),
                    start_line: i,
                    end_line: i,
                    confidence: 0.95,
                });
                i += 1;
                continue;
            }

            // Table — collect consecutive table lines
            if self.rules.is_table(line.trim()) {
                let start = i;
                let mut table_lines = vec![line.to_string()];
                i += 1;
                while i < lines.len() && self.rules.is_table(lines[i].trim()) {
                    table_lines.push(lines[i].to_string());
                    i += 1;
                }
                regions.push(LayoutRegion {
                    element: LayoutElement::Table,
                    text: table_lines.join("\n"),
                    start_line: start,
                    end_line: i - 1,
                    confidence: 0.95,
                });
                continue;
            }

            // List (unordered)
            if self.rules.is_unordered_list(line.trim()) {
                let start = i;
                let mut list_lines = vec![line.to_string()];
                i += 1;
                while i < lines.len() && self.rules.is_unordered_list(lines[i].trim()) {
                    list_lines.push(lines[i].to_string());
                    i += 1;
                }
                regions.push(LayoutRegion {
                    element: LayoutElement::List { ordered: false },
                    text: list_lines.join("\n"),
                    start_line: start,
                    end_line: i - 1,
                    confidence: 0.95,
                });
                continue;
            }

            // List (ordered)
            if self.rules.is_ordered_list(line.trim()) {
                let start = i;
                let mut list_lines = vec![line.to_string()];
                i += 1;
                while i < lines.len() && self.rules.is_ordered_list(lines[i].trim()) {
                    list_lines.push(lines[i].to_string());
                    i += 1;
                }
                regions.push(LayoutRegion {
                    element: LayoutElement::List { ordered: true },
                    text: list_lines.join("\n"),
                    start_line: start,
                    end_line: i - 1,
                    confidence: 0.9,
                });
                continue;
            }

            // Quote
            if self.rules.is_quote(line.trim()) {
                let start = i;
                let mut quote_lines = vec![line.to_string()];
                i += 1;
                while i < lines.len() && self.rules.is_quote(lines[i].trim()) {
                    quote_lines.push(lines[i].to_string());
                    i += 1;
                }
                regions.push(LayoutRegion {
                    element: LayoutElement::Quote,
                    text: quote_lines.join("\n"),
                    start_line: start,
                    end_line: i - 1,
                    confidence: 0.95,
                });
                continue;
            }

            // Paragraph — collect consecutive non-empty, non-special lines
            let start = i;
            let mut para_lines = vec![line.to_string()];
            i += 1;
            while i < lines.len() {
                let next = lines[i].trim();
                if next.is_empty()
                    || self.rules.match_heading(next).is_some()
                    || self.rules.match_code_fence(next).is_some()
                    || self.rules.is_table(next)
                    || self.rules.is_unordered_list(next)
                    || self.rules.is_ordered_list(next)
                    || self.rules.is_quote(next)
                    || self.rules.is_horizontal_rule(next)
                    || self.rules.match_image(next).is_some()
                {
                    break;
                }
                para_lines.push(lines[i].to_string());
                i += 1;
            }
            regions.push(LayoutRegion {
                element: LayoutElement::Paragraph,
                text: para_lines.join("\n"),
                start_line: start,
                end_line: i - 1,
                confidence: 0.85,
            });
        }

        regions
    }

    /// Group regions into sections by heading hierarchy.
    pub fn extract_sections(&self, text: &str) -> Vec<DocSection> {
        let regions = self.analyze(text);
        Self::build_sections(&regions, 0)
    }

    /// Recursively build section tree from flat regions.
    fn build_sections(regions: &[LayoutRegion], _min_level: u8) -> Vec<DocSection> {
        let mut sections: Vec<DocSection> = Vec::new();
        let mut i = 0;

        // Collect any regions before the first heading into a root section
        let mut preamble: Vec<LayoutRegion> = Vec::new();
        while i < regions.len() {
            if matches!(regions[i].element, LayoutElement::Heading(_)) {
                break;
            }
            preamble.push(regions[i].clone());
            i += 1;
        }
        if !preamble.is_empty() {
            sections.push(DocSection {
                heading: String::new(),
                level: 0,
                regions: preamble,
                children: Vec::new(),
            });
        }

        while i < regions.len() {
            if let LayoutElement::Heading(level) = &regions[i].element {
                let level = *level;
                let heading_text = regions[i].text.clone();
                let mut child_regions: Vec<LayoutRegion> = Vec::new();
                let mut sub_heading_regions: Vec<LayoutRegion> = Vec::new();

                i += 1;
                // Collect all regions that belong under this heading
                while i < regions.len() {
                    if let LayoutElement::Heading(next_level) = &regions[i].element {
                        if *next_level <= level {
                            break; // same or higher level heading — end of section
                        }
                    }
                    // Separate non-heading content from sub-headings
                    if matches!(regions[i].element, LayoutElement::Heading(_)) {
                        // This is a deeper heading — collect it and everything under it for children
                        sub_heading_regions.push(regions[i].clone());
                    } else if sub_heading_regions.is_empty() {
                        child_regions.push(regions[i].clone());
                    } else {
                        sub_heading_regions.push(regions[i].clone());
                    }
                    i += 1;
                }

                let children = if sub_heading_regions.is_empty() {
                    Vec::new()
                } else {
                    Self::build_sections(&sub_heading_regions, level + 1)
                };

                sections.push(DocSection {
                    heading: heading_text,
                    level,
                    regions: child_regions,
                    children,
                });
            } else {
                i += 1;
            }
        }

        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_detection_h1() {
        let analyzer = LayoutAnalyzer::new();
        let regions = analyzer.analyze("# Hello World");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::Heading(1));
        assert_eq!(regions[0].text, "Hello World");
        assert_eq!(regions[0].confidence, 1.0);
    }

    #[test]
    fn test_heading_detection_h3() {
        let analyzer = LayoutAnalyzer::new();
        let regions = analyzer.analyze("### Sub-heading");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::Heading(3));
        assert_eq!(regions[0].text, "Sub-heading");
    }

    #[test]
    fn test_code_block_with_language() {
        let analyzer = LayoutAnalyzer::new();
        let text = "```rust\nfn main() {}\n```";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0].element,
            LayoutElement::CodeBlock(Some("rust".to_string()))
        );
        assert_eq!(regions[0].text, "fn main() {}");
    }

    #[test]
    fn test_code_block_no_language() {
        let analyzer = LayoutAnalyzer::new();
        let text = "```\nhello world\n```";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::CodeBlock(None));
    }

    #[test]
    fn test_table_detection() {
        let analyzer = LayoutAnalyzer::new();
        let text = "| Col1 | Col2 |\n| --- | --- |\n| A | B |";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::Table);
        assert_eq!(regions[0].start_line, 0);
        assert_eq!(regions[0].end_line, 2);
    }

    #[test]
    fn test_unordered_list() {
        let analyzer = LayoutAnalyzer::new();
        let text = "- item one\n- item two\n- item three";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::List { ordered: false });
    }

    #[test]
    fn test_ordered_list() {
        let analyzer = LayoutAnalyzer::new();
        let text = "1. first\n2. second\n3. third";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::List { ordered: true });
    }

    #[test]
    fn test_quote_detection() {
        let analyzer = LayoutAnalyzer::new();
        let text = "> This is a quote\n> Second line";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::Quote);
    }

    #[test]
    fn test_horizontal_rule() {
        let analyzer = LayoutAnalyzer::new();
        let regions = analyzer.analyze("---");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::HorizontalRule);

        let regions2 = analyzer.analyze("***");
        assert_eq!(regions2.len(), 1);
        assert_eq!(regions2[0].element, LayoutElement::HorizontalRule);
    }

    #[test]
    fn test_mixed_content() {
        let analyzer = LayoutAnalyzer::new();
        let text = "# Title\n\nSome paragraph text.\n\n```python\nprint('hello')\n```\n\n- item 1\n- item 2\n\n---\n\n> A quote";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 6);
        assert_eq!(regions[0].element, LayoutElement::Heading(1));
        assert_eq!(regions[1].element, LayoutElement::Paragraph);
        assert_eq!(
            regions[2].element,
            LayoutElement::CodeBlock(Some("python".to_string()))
        );
        assert_eq!(regions[3].element, LayoutElement::List { ordered: false });
        assert_eq!(regions[4].element, LayoutElement::HorizontalRule);
        assert_eq!(regions[5].element, LayoutElement::Quote);
    }

    #[test]
    fn test_empty_input() {
        let analyzer = LayoutAnalyzer::new();
        let regions = analyzer.analyze("");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_nested_headings_sections() {
        let analyzer = LayoutAnalyzer::new();
        let text = "# Main\n\nIntro text.\n\n## Sub1\n\nSub1 content.\n\n## Sub2\n\nSub2 content.";
        let sections = analyzer.extract_sections(text);
        assert_eq!(sections.len(), 1); // one top-level section
        assert_eq!(sections[0].heading, "Main");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[0].children.len(), 2);
        assert_eq!(sections[0].children[0].heading, "Sub1");
        assert_eq!(sections[0].children[1].heading, "Sub2");
    }

    #[test]
    fn test_multiline_paragraph() {
        let analyzer = LayoutAnalyzer::new();
        let text = "This is line one.\nThis is line two.\nThis is line three.";
        let regions = analyzer.analyze(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].element, LayoutElement::Paragraph);
        assert!(regions[0].text.contains("line one"));
        assert!(regions[0].text.contains("line three"));
        assert_eq!(regions[0].start_line, 0);
        assert_eq!(regions[0].end_line, 2);
    }

    #[test]
    fn test_image_detection() {
        let analyzer = LayoutAnalyzer::new();
        let regions = analyzer.analyze("![My image](https://example.com/img.png)");
        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0].element,
            LayoutElement::Image(Some("My image".to_string()))
        );
    }
}
