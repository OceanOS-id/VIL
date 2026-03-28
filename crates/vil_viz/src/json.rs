//! JSON output — structured graph data for egui IDE and custom tooling.

use crate::graph::VizGraph;

pub fn render(graph: &VizGraph) -> Result<String, String> {
    serde_json::to_string_pretty(graph).map_err(|e| format!("JSON serialization failed: {}", e))
}
