/// Trait for document parsers that clean raw content into plain text.
pub trait DocumentParser: Send + Sync {
    /// Parse raw content into cleaned text suitable for chunking.
    fn parse(&self, content: &str) -> String;

    /// File types this parser supports (e.g., "txt", "md").
    fn supported_types(&self) -> Vec<&str>;
}

// ---------------------------------------------------------------------------
// PlainTextParser
// ---------------------------------------------------------------------------

/// Passes text through with minimal cleanup (trim whitespace, normalize newlines).
pub struct PlainTextParser;

impl DocumentParser for PlainTextParser {
    fn parse(&self, content: &str) -> String {
        content
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    fn supported_types(&self) -> Vec<&str> {
        vec!["txt", "text"]
    }
}

// ---------------------------------------------------------------------------
// MarkdownParser — strips markdown syntax, keeps text
// ---------------------------------------------------------------------------

/// Strips common markdown syntax (headers `#`, bold `**`, italic `*`, links,
/// code fences, inline code) and returns plain text.
pub struct MarkdownParser;

impl DocumentParser for MarkdownParser {
    fn parse(&self, content: &str) -> String {
        let mut result = Vec::new();
        let mut in_code_fence = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Toggle code fences
            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }
            if in_code_fence {
                // Keep code content as-is
                result.push(line.to_string());
                continue;
            }

            // Strip header markers
            let line = if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim().to_string()
            } else {
                line.to_string()
            };

            // Strip inline formatting
            let line = line
                .replace("**", "")
                .replace("__", "")
                .replace('*', "")
                .replace('_', " ");

            // Strip links: [text](url) -> text
            let line = strip_md_links(&line);

            // Strip inline code backticks
            let line = line.replace('`', "");

            // Strip horizontal rules
            let trimmed = line.trim();
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                continue;
            }

            result.push(line);
        }

        result.join("\n").trim().to_string()
    }

    fn supported_types(&self) -> Vec<&str> {
        vec!["md", "markdown"]
    }
}

/// Strip markdown links `[text](url)` -> `text`.
fn strip_md_links(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '[' {
            // Find closing ]
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let close_bracket = i + 1 + close_bracket;
                // Check for (url) after ]
                if close_bracket + 1 < len && chars[close_bracket + 1] == '(' {
                    if let Some(close_paren) =
                        chars[close_bracket + 2..].iter().position(|&c| c == ')')
                    {
                        // Extract link text
                        let text: String = chars[i + 1..close_bracket].iter().collect();
                        result.push_str(&text);
                        i = close_bracket + 2 + close_paren + 1;
                        continue;
                    }
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_parser_trims() {
        let parser = PlainTextParser;
        let result = parser.parse("  hello  \n  world  \n");
        assert_eq!(result, "hello\n  world");
    }

    #[test]
    fn markdown_parser_strips_headers() {
        let parser = MarkdownParser;
        let result = parser.parse("# Title\n## Subtitle\nBody text.");
        assert!(result.contains("Title"));
        assert!(result.contains("Subtitle"));
        assert!(result.contains("Body text."));
        assert!(!result.contains('#'));
    }

    #[test]
    fn markdown_parser_strips_links() {
        let parser = MarkdownParser;
        let result = parser.parse("Check [this link](https://example.com) out.");
        assert_eq!(result, "Check this link out.");
    }

    #[test]
    fn markdown_parser_strips_bold_italic() {
        let parser = MarkdownParser;
        let result = parser.parse("This is **bold** and *italic* text.");
        assert!(!result.contains('*'));
    }
}
