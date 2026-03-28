//! Visualization configuration.

/// Output format.
#[derive(Debug, Clone, PartialEq)]
pub enum VizFormat {
    Html,
    Svg,
    Mermaid,
    Dot,
    Json,
    Ascii,
}

impl VizFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "html" => Ok(VizFormat::Html),
            "svg" => Ok(VizFormat::Svg),
            "mermaid" | "md" => Ok(VizFormat::Mermaid),
            "dot" | "graphviz" => Ok(VizFormat::Dot),
            "json" => Ok(VizFormat::Json),
            "ascii" | "text" | "txt" => Ok(VizFormat::Ascii),
            _ => Err(format!(
                "Unknown format '{}'. Supported: html, svg, mermaid, dot, json, ascii",
                s
            )),
        }
    }
}

/// Zoom level.
#[derive(Debug, Clone, PartialEq)]
pub enum VizLevel {
    Topology,
    Dag,
    Full,
}

impl VizLevel {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "topology" | "topo" => Ok(VizLevel::Topology),
            "dag" | "workflow" => Ok(VizLevel::Dag),
            "full" | "all" => Ok(VizLevel::Full),
            _ => Err(format!(
                "Unknown level '{}'. Supported: topology, dag, full",
                s
            )),
        }
    }
}

/// Display configuration for visualization.
#[derive(Debug, Clone)]
pub struct VizConfig {
    pub format: VizFormat,
    pub level: VizLevel,
    pub show_lanes: bool,
    pub show_topology: bool,
    pub show_ports: bool,
    pub show_messages: bool,
    pub show_workflows: bool,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            format: VizFormat::Html,
            level: VizLevel::Topology,
            show_lanes: false,
            show_topology: false,
            show_ports: false,
            show_messages: false,
            show_workflows: false,
        }
    }
}
