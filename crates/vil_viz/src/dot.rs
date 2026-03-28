//! Graphviz DOT output — for large graphs and pipeline visualization.

use crate::config::VizConfig;
use crate::graph::{VizGraph, VizNodeType};

pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&format!("digraph \"{}\" {{\n", graph.name));
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [fontname=\"Helvetica\", fontsize=11];\n");
    out.push_str("  edge [fontname=\"Helvetica\", fontsize=9];\n\n");

    // Nodes
    for node in &graph.nodes {
        let (shape, color) = match node.node_type {
            VizNodeType::Sink => ("box", "#4CAF50"),
            VizNodeType::Source => ("ellipse", "#2196F3"),
            VizNodeType::Transform => ("diamond", "#FF9800"),
            VizNodeType::Task => ("box", "#9E9E9E"),
            VizNodeType::Branch => ("diamond", "#E91E63"),
            VizNodeType::Switch => ("diamond", "#E91E63"),
            VizNodeType::Merge => ("hexagon", "#9C27B0"),
            VizNodeType::Wasm => ("box3d", "#FF5722"),
        };
        let mut label = node.label.clone();
        if config.show_topology {
            if let Some(host) = &node.host {
                label = format!("{}\\n@{}", label, host);
            }
        }
        out.push_str(&format!(
            "  \"{}\" [label=\"{}\", shape={}, style=filled, fillcolor=\"{}\", fontcolor=white];\n",
            node.id, label, shape, color
        ));
    }
    out.push('\n');

    // Edges
    for edge in &graph.edges {
        let mut label_parts = Vec::new();
        if config.show_lanes {
            if let Some(lane) = &edge.lane {
                label_parts.push(lane.clone());
            }
        }
        if let Some(mode) = &edge.mode {
            label_parts.push(mode.clone());
        }
        let label = label_parts.join("\\n");
        let detach_style = if edge.detach {
            ", style=dashed, color=gray"
        } else {
            ""
        };
        if label.is_empty() {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" [{}];\n",
                edge.from_node,
                edge.to_node,
                detach_style.trim_start_matches(", ")
            ));
        } else {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"{}];\n",
                edge.from_node, edge.to_node, label, detach_style
            ));
        }
    }

    // Subgraphs
    if config.show_workflows {
        for sg in &graph.subgraphs {
            out.push_str(&format!("\n  subgraph \"cluster_{}\" {{\n", sg.parent_node));
            out.push_str(&format!("    label=\"{} DAG\";\n", sg.parent_node));
            out.push_str("    style=dashed;\n");
            out.push_str("    color=\"#666666\";\n");
            for node in &sg.nodes {
                out.push_str(&format!(
                    "    \"{}\" [label=\"{}\", shape=box];\n",
                    node.id, node.label
                ));
            }
            for edge in &sg.edges {
                out.push_str(&format!(
                    "    \"{}\" -> \"{}\";\n",
                    edge.from_node, edge.to_node
                ));
            }
            out.push_str("  }\n");
        }
    }

    out.push_str("}\n");
    Ok(out)
}
