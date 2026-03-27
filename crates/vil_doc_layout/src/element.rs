//! Layout element types for document analysis.

use serde::{Deserialize, Serialize};

/// Represents the type of layout element detected in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutElement {
    /// Heading with level (1-6).
    Heading(u8),
    /// Plain paragraph text.
    Paragraph,
    /// Code block with optional language identifier.
    CodeBlock(Option<String>),
    /// Table content.
    Table,
    /// List (ordered or unordered).
    List { ordered: bool },
    /// Image with optional caption.
    Image(Option<String>),
    /// Block quote.
    Quote,
    /// Horizontal rule / thematic break.
    HorizontalRule,
}

/// A detected region in the document with its layout element type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRegion {
    /// The type of layout element.
    pub element: LayoutElement,
    /// The raw text content of this region.
    pub text: String,
    /// Starting line number (0-indexed).
    pub start_line: usize,
    /// Ending line number (0-indexed, inclusive).
    pub end_line: usize,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
}

/// A document section grouped by heading hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSection {
    /// Section heading text (empty for root).
    pub heading: String,
    /// Heading level (0 for root/top-level content before any heading).
    pub level: u8,
    /// All regions within this section (excluding the heading itself).
    pub regions: Vec<LayoutRegion>,
    /// Nested sub-sections.
    pub children: Vec<DocSection>,
}
