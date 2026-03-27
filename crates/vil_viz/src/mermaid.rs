//! Mermaid diagram output — renders in GitHub, Notion, and Mermaid Live Editor.

use crate::config::VizConfig;
use crate::graph::{VizEdge, VizGraph, VizNodeType, VizSubgraph};

pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("graph LR\n");

    // Render topology nodes
    for node in &graph.nodes {
        let shape = mermaid_node(&node.id, &node.label, &node.node_type, config);
        out.push_str(&format!("  {}\n", shape));
    }
    out.push('\n');

    // Render edges (solid = attached, dashed = detached)
    for edge in &graph.edges {
        let label = edge_label(edge, config);
        if label.is_empty() {
            if edge.detach {
                out.push_str(&format!("  {} -.-> {}\n", edge.from_node, edge.to_node));
            } else {
                out.push_str(&format!("  {} --> {}\n", edge.from_node, edge.to_node));
            }
        } else {
            if edge.detach {
                out.push_str(&format!("  {} -.->|{}| {}\n", edge.from_node, label, edge.to_node));
            } else {
                out.push_str(&format!("  {} -->|{}| {}\n", edge.from_node, label, edge.to_node));
            }
        }
    }

    // Render subgraphs (workflow DAGs inside nodes)
    if config.show_workflows {
        for sg in &graph.subgraphs {
            out.push('\n');
            render_subgraph(&mut out, sg, config);
        }
    }

    Ok(out)
}

fn mermaid_node(id: &str, label: &str, node_type: &VizNodeType, config: &VizConfig) -> String {
    let host_suffix = ""; // TODO: add host info if config.show_topology
    let _ = config;
    let content = format!("{}{}", label, host_suffix);
    match node_type {
        VizNodeType::Sink => format!("{}[[\"{}\"]]", id, content),
        VizNodeType::Source => format!("{}((\"{}\" ))", id, content),
        VizNodeType::Transform => format!("{}[/\"{}\"\\]", id, content),
        VizNodeType::Task => format!("{}[\"{}\"]", id, content),
        VizNodeType::Branch => {
            // Mermaid diamond: {label}
            let mut s = String::new();
            s.push_str(id);
            s.push('{');
            s.push('"');
            s.push_str(&content);
            s.push('"');
            s.push('}');
            s
        }
        VizNodeType::Switch => {
            // Mermaid diamond for switch too
            let mut s = String::new();
            s.push_str(id);
            s.push('{');
            s.push('"');
            s.push_str(&content);
            s.push('"');
            s.push('}');
            s
        }
        VizNodeType::Merge => {
            let mut s = String::new();
            s.push_str(id);
            s.push_str("{{\"");
            s.push_str(&content);
            s.push_str("\"}}");
            s
        }
        VizNodeType::Wasm => format!("{}([\"{}\" ])", id, content),
    }
}

fn edge_label(edge: &VizEdge, config: &VizConfig) -> String {
    let mut parts = Vec::new();
    if config.show_lanes {
        if let Some(lane) = &edge.lane {
            parts.push(lane.clone());
        }
    }
    if let Some(mode) = &edge.mode {
        if mode != "LoanWrite" {
            parts.push(mode.clone());
        }
    }
    if config.show_messages {
        if let Some(msg) = &edge.message_type {
            parts.push(msg.clone());
        }
    }
    parts.join(" ")
}

fn render_subgraph(out: &mut String, sg: &VizSubgraph, config: &VizConfig) {
    out.push_str(&format!("  subgraph {}_dag [\"{}\"]\n", sg.parent_node, sg.parent_node));
    for node in &sg.nodes {
        let shape = mermaid_node(&node.id, &node.label, &node.node_type, config);
        out.push_str(&format!("    {}\n", shape));
    }
    for edge in &sg.edges {
        let label = edge_label(edge, config);
        if label.is_empty() {
            out.push_str(&format!("    {} --> {}\n", edge.from_node, edge.to_node));
        } else {
            out.push_str(&format!("    {} -->|{}| {}\n", edge.from_node, label, edge.to_node));
        }
    }
    out.push_str("  end\n");
}
