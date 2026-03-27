//! ASCII art output — terminal-friendly for SSH/CI environments.

use crate::config::VizConfig;
use crate::graph::VizGraph;

pub fn render(graph: &VizGraph, config: &VizConfig) -> Result<String, String> {
    let mut out = String::new();

    out.push_str(&format!("╔══ {} ", graph.name));
    let remaining = 60usize.saturating_sub(graph.name.len() + 5);
    for _ in 0..remaining { out.push('═'); }
    out.push_str("╗\n");

    // Nodes section
    out.push_str("║ NODES:\n");
    for node in &graph.nodes {
        let icon = node.node_type.icon();
        let host_info = if config.show_topology {
            node.host.as_deref().map(|h| format!(" @{}", h)).unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!("║   [{}] {}{}\n", icon, node.label, host_info));

        if config.show_ports {
            for port in &node.ports {
                let dir = if port.direction == "in" { "◀" } else { "▶" };
                let msg = port.message_type.as_deref().unwrap_or("");
                let lane = port.lane.as_deref().unwrap_or("");
                out.push_str(&format!("║       {} {} {} {}\n", dir, port.name, lane, msg));
            }
        }
    }

    // Routes section
    out.push_str("║\n║ ROUTES:\n");
    for edge in &graph.edges {
        let from = if let Some(p) = &edge.from_port {
            format!("{}.{}", edge.from_node, p)
        } else {
            edge.from_node.clone()
        };
        let to = if let Some(p) = &edge.to_port {
            format!("{}.{}", edge.to_node, p)
        } else {
            edge.to_node.clone()
        };
        let mode = edge.mode.as_deref().unwrap_or("--");
        let lane = if config.show_lanes {
            edge.lane.as_deref().unwrap_or("")
        } else { "" };
        let arrow = if edge.detach { "~~detach~~▶" } else { &format!("──{}──▶", mode) };
        out.push_str(&format!("║   {} {} {} {}\n", from, arrow, to, lane));
    }

    // Subgraphs
    if config.show_workflows && !graph.subgraphs.is_empty() {
        out.push_str("║\n║ WORKFLOW DAGs:\n");
        for sg in &graph.subgraphs {
            out.push_str(&format!("║   ┌── {} ──────────────────────────\n", sg.parent_node));
            for node in &sg.nodes {
                let icon = node.node_type.icon();
                out.push_str(&format!("║   │ [{}] {}\n", icon, node.label));
            }
            for edge in &sg.edges {
                out.push_str(&format!("║   │   {} ──▶ {}\n", edge.from_node, edge.to_node));
            }
            out.push_str("║   └─────────────────────────────────\n");
        }
    }

    // Summary
    out.push_str(&format!(
        "║\n║ {} nodes, {} routes",
        graph.nodes.len(), graph.edges.len()
    ));
    if !graph.subgraphs.is_empty() {
        out.push_str(&format!(", {} workflow DAGs", graph.subgraphs.len()));
    }
    out.push('\n');
    out.push_str("╚");
    for _ in 0..64 { out.push('═'); }
    out.push_str("╝\n");

    Ok(out)
}
