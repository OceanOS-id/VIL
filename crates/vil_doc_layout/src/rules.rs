//! Rule-based layout detection logic.

use regex::Regex;

/// Compiled rule patterns for layout detection.
pub(crate) struct LayoutRules {
    pub heading_re: Regex,
    pub ordered_list_re: Regex,
    pub unordered_list_re: Regex,
    pub table_re: Regex,
    pub quote_re: Regex,
    pub hr_re: Regex,
    pub code_fence_re: Regex,
    pub image_re: Regex,
}

impl LayoutRules {
    pub fn new() -> Self {
        Self {
            heading_re: Regex::new(r"^(#{1,6})\s+(.*)$").unwrap(),
            ordered_list_re: Regex::new(r"^\d+\.\s+").unwrap(),
            unordered_list_re: Regex::new(r"^-\s+").unwrap(),
            table_re: Regex::new(r"^\|").unwrap(),
            quote_re: Regex::new(r"^>\s?").unwrap(),
            hr_re: Regex::new(r"^(---+|\*\*\*+)$").unwrap(),
            code_fence_re: Regex::new(r"^```(.*)$").unwrap(),
            image_re: Regex::new(r"^!\[([^\]]*)\]\(([^)]+)\)$").unwrap(),
        }
    }

    /// Detect heading and return (level, text) if matched.
    pub fn match_heading<'a>(&self, line: &'a str) -> Option<(u8, &'a str)> {
        self.heading_re.captures(line).map(|caps| {
            let level = caps.get(1).unwrap().as_str().len() as u8;
            let text = caps.get(2).unwrap().as_str();
            (level, text)
        })
    }

    /// Detect code fence and return the language tag (may be empty).
    pub fn match_code_fence<'a>(&self, line: &'a str) -> Option<&'a str> {
        self.code_fence_re
            .captures(line)
            .map(|caps| caps.get(1).unwrap().as_str().trim())
    }

    /// Detect image and return optional caption.
    pub fn match_image<'a>(&self, line: &'a str) -> Option<Option<&'a str>> {
        self.image_re.captures(line).map(|caps| {
            let alt = caps.get(1).unwrap().as_str();
            if alt.is_empty() {
                None
            } else {
                Some(alt)
            }
        })
    }

    pub fn is_ordered_list(&self, line: &str) -> bool {
        self.ordered_list_re.is_match(line)
    }

    pub fn is_unordered_list(&self, line: &str) -> bool {
        self.unordered_list_re.is_match(line)
    }

    pub fn is_table(&self, line: &str) -> bool {
        self.table_re.is_match(line)
    }

    pub fn is_quote(&self, line: &str) -> bool {
        self.quote_re.is_match(line)
    }

    pub fn is_horizontal_rule(&self, line: &str) -> bool {
        self.hr_re.is_match(line)
    }
}
