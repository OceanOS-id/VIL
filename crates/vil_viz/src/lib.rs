//! VIL Workflow Visualization Engine
//!
//! Pure rendering library: takes a `VizGraph` and outputs 6 formats.
//! Does NOT depend on WorkflowManifest — conversion happens in the caller.
//!
//! Formats:
//!   - Mermaid: GitHub/Notion-compatible diagrams
//!   - JSON: structured data for egui IDE
//!   - DOT: Graphviz for large graphs
//!   - ASCII: terminal-friendly box drawing
//!   - SVG: vector graphics via `dot -Tsvg`
//!   - HTML: self-contained interactive viewer

pub mod graph;
pub mod config;
pub mod mermaid;
pub mod json;
pub mod dot;
pub mod ascii;
pub mod svg;
pub mod html;

pub use graph::*;
pub use config::*;

/// Render a VizGraph to the specified output format.
pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    match config.format {
        VizFormat::Mermaid => mermaid::render(graph, config),
        VizFormat::Json => json::render(graph),
        VizFormat::Dot => dot::render(graph, config),
        VizFormat::Ascii => ascii::render(graph, config),
        VizFormat::Svg => svg::render(graph, config),
        VizFormat::Html => html::render(graph, config),
    }
}
