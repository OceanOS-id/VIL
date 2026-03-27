//! VIL Document Layout Analysis (H06).
//!
//! Rule-based layout detection from Markdown-like text documents.
//! Identifies headings, code blocks, tables, lists, quotes, images, and paragraphs.
//!
//! ```
//! use vil_doc_layout::{LayoutAnalyzer, LayoutElement};
//!
//! let analyzer = LayoutAnalyzer::new();
//! let regions = analyzer.analyze("# Hello\n\nSome text.");
//! assert_eq!(regions[0].element, LayoutElement::Heading(1));
//! ```

pub mod analyzer;
pub mod element;
mod rules;

pub use analyzer::LayoutAnalyzer;
pub use element::{DocSection, LayoutElement, LayoutRegion};

// VIL integration layer
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::DocLayoutPlugin;
pub use semantic::{LayoutAnalyzeEvent, LayoutFault, DocLayoutState};
