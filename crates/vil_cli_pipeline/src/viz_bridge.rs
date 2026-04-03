//! Bridge between WorkflowManifest and vil_viz rendering engine.
//!
//! Converts manifest data into VizGraph, then dispatches to vil_viz::render().

use vil_cli_core::manifest::WorkflowManifest;
use vil_viz::{
    VizConfig, VizEdge, VizFormat, VizGraph, VizLevel, VizNode, VizNodeType, VizPort, VizSubgraph,
};

pub struct VizArgs {
    pub input: String,
    pub format: String,
    pub output: Option<String>,
    pub show_lanes: bool,
    pub show_topology: bool,
    pub show_ports: bool,
    pub show_messages: bool,
    pub show_workflows: bool,
    pub level: String,
    pub open: bool,
    pub call_graph: Option<String>,
    pub expand_calls: bool,
}

pub fn run_viz(args: VizArgs) -> Result<(), String> {
    // If --call-graph is set, build a file-level graph instead
    if let Some(dir) = &args.call_graph {
        return run_call_graph_viz(dir, &args);
    }

    // 1. Parse YAML — try WorkflowManifest first
    let manifest = WorkflowManifest::from_file(&args.input)?;

    // 2. Convert to VizGraph (with optional call expansion)
    let mut graph = manifest_to_graph(&manifest, &args);

    // If --expand-calls, resolve call: targets and inline as subgraphs
    if args.expand_calls {
        let base_dir = std::path::Path::new(&args.input)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        if let Ok(resolved) = vil_cli_compile::call_resolver::resolve_all_calls(&manifest, base_dir) {
            for (call_path, resolved_call) in &resolved {
                let sub = call_to_subgraph(call_path, resolved_call);
                graph.subgraphs.push(sub);
            }
        }
    }

    // 3. Build config
    let config = VizConfig {
        format: VizFormat::from_str(&args.format)?,
        level: VizLevel::from_str(&args.level)?,
        show_lanes: args.show_lanes,
        show_topology: args.show_topology,
        show_ports: args.show_ports,
        show_messages: args.show_messages,
        show_workflows: args.show_workflows,
    };

    // 4. Render
    let output = vil_viz::render(&graph, &config)?;

    // 5. Write to file, open browser, or stdout
    if args.open {
        // --open: write to file and open browser, no stdout
        let path = args.output.as_deref().unwrap_or("/tmp/vil-viz.html");
        std::fs::write(path, &output).map_err(|e| format!("Write failed: {}", e))?;
        eprintln!("Written to {}", path);
        open_browser(path);
    } else if let Some(path) = &args.output {
        // --output: write to file only
        std::fs::write(path, &output).map_err(|e| format!("Write failed: {}", e))?;
        eprintln!("Written to {}", path);
    } else {
        // No --open, no --output: print to stdout
        println!("{}", output);
    }

    Ok(())
}

fn manifest_to_graph(manifest: &WorkflowManifest, args: &VizArgs) -> VizGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut subgraphs = Vec::new();

    // Convert nodes
    for (name, node) in &manifest.nodes {
        let node_type = match node.node_type.as_str() {
            "http-sink" => VizNodeType::Sink,
            "http-source" => VizNodeType::Source,
            "transform" => {
                if node.code.as_ref().map(|c| c.mode.as_str()) == Some("wasm") {
                    VizNodeType::Wasm
                } else {
                    VizNodeType::Transform
                }
            }
            other => {
                // Check if it's a registered AI/DB node type
                if vil_cli_core::node_types::find_node_type(other).is_some() {
                    VizNodeType::Transform // AI/DB nodes rendered as Transform shape
                } else {
                    VizNodeType::Task
                }
            }
        };

        let mut label = name.clone();
        if let Some(port) = node.port {
            label = format!("{}\\n:{}", name, port);
        }
        if node.node_type == "http-source" {
            if let Some(url) = &node.url {
                // Show shortened URL
                let short = if url.len() > 30 { &url[..30] } else { url };
                label = format!("{}\\n{}", name, short);
            }
        }

        let mut ports = Vec::new();
        for (port_name, port_def) in &node.ports {
            ports.push(VizPort {
                name: port_name.clone(),
                direction: port_def.direction.clone(),
                message_type: port_def.message.clone(),
                lane: port_def.lane.clone(),
            });
        }

        let host = manifest
            .topology
            .as_ref()
            .and_then(|t| t.placement.get(name).map(|p| p.host.clone()));

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("type".into(), node.node_type.clone());
        metadata.insert("exec_class".into(), node.exec_class.clone());
        // Add category and color from node type registry
        if let Some(entry) = vil_cli_core::node_types::find_node_type(&node.node_type) {
            metadata.insert("category".into(), entry.category.into());
            metadata.insert("color".into(), entry.color.into());
        }
        if let Some(code) = &node.code {
            metadata.insert("code_mode".into(), code.mode.clone());
            if let Some(handler) = &code.handler {
                metadata.insert("handler".into(), handler.clone());
            }
            if let Some(expr) = &code.expr {
                metadata.insert("expr".into(), expr.clone());
            }
        }

        nodes.push(VizNode {
            id: name.clone(),
            label,
            node_type,
            ports,
            host,
            metadata,
        });
    }

    // Convert routes
    for route in &manifest.workflow_routes {
        let from_parts: Vec<&str> = route.from.splitn(2, '.').collect();
        let to_parts: Vec<&str> = route.to.splitn(2, '.').collect();

        edges.push(VizEdge {
            from_node: from_parts.first().unwrap_or(&"").to_string(),
            from_port: from_parts.get(1).map(|s| s.to_string()),
            to_node: to_parts.first().unwrap_or(&"").to_string(),
            to_port: to_parts.get(1).map(|s| s.to_string()),
            lane: None,
            mode: Some(route.mode.clone()),
            message_type: None,
            detach: route.detach.unwrap_or(false),
        });
    }

    // Convert workflow DAGs to subgraphs
    if args.show_workflows {
        for (wf_name, wf) in &manifest.workflows {
            let mut sg_nodes = Vec::new();
            let mut sg_edges = Vec::new();

            for task in &wf.tasks {
                let node_type = match task.task_type.as_deref().unwrap_or("Transform") {
                    "Branch" => VizNodeType::Branch,
                    "Merge" => VizNodeType::Merge,
                    _ => VizNodeType::Task,
                };
                sg_nodes.push(VizNode {
                    id: task.id.clone(),
                    label: task.name.as_deref().unwrap_or(&task.id).to_string(),
                    node_type,
                    ports: Vec::new(),
                    host: None,
                    metadata: std::collections::HashMap::new(),
                });

                for dep in &task.deps {
                    sg_edges.push(VizEdge {
                        from_node: dep.task_id().to_string(),
                        from_port: None,
                        to_node: task.id.clone(),
                        to_port: None,
                        lane: None,
                        mode: None,
                        message_type: None,
                        detach: dep.is_detached(),
                    });
                }
            }

            for branch in &wf.branches {
                sg_nodes.push(VizNode {
                    id: branch.id.clone(),
                    label: branch.name.as_deref().unwrap_or(&branch.id).to_string(),
                    node_type: match branch.branch_type.as_str() {
                        "Branch" => VizNodeType::Branch,
                        "Switch" => VizNodeType::Switch,
                        _ => VizNodeType::Task,
                    },
                    ports: Vec::new(),
                    host: None,
                    metadata: std::collections::HashMap::new(),
                });

                for dep in &branch.deps {
                    sg_edges.push(VizEdge {
                        from_node: dep.task_id().to_string(),
                        from_port: None,
                        to_node: branch.id.clone(),
                        to_port: None,
                        lane: None,
                        mode: None,
                        message_type: None,
                        detach: dep.is_detached(),
                    });
                }
                if let Some(on_true) = &branch.on_true {
                    sg_edges.push(VizEdge {
                        from_node: branch.id.clone(),
                        from_port: None,
                        to_node: on_true.clone(),
                        to_port: None,
                        lane: Some("true".into()),
                        mode: None,
                        message_type: None,
                        detach: false,
                    });
                }
                if let Some(on_false) = &branch.on_false {
                    sg_edges.push(VizEdge {
                        from_node: branch.id.clone(),
                        from_port: None,
                        to_node: on_false.clone(),
                        to_port: None,
                        lane: Some("false".into()),
                        mode: None,
                        message_type: None,
                        detach: false,
                    });
                }
            }

            subgraphs.push(VizSubgraph {
                parent_node: wf_name.clone(),
                nodes: sg_nodes,
                edges: sg_edges,
            });
        }
    }

    VizGraph {
        name: manifest.name.clone(),
        nodes,
        edges,
        subgraphs,
    }
}

fn open_browser(path: &str) {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", path])
        .spawn();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Call graph visualization
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a file-level call graph: each .vil.yaml file is a node, call: references are edges.
fn run_call_graph_viz(dir: &str, args: &VizArgs) -> Result<(), String> {
    let call_graph = vil_cli_compile::call_resolver::scan_call_graph(std::path::Path::new(dir))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for node in &call_graph.nodes {
        let label = format!(
            "{}\\n{} nodes, {} wf",
            node.name, node.node_count, node.workflow_count
        );
        nodes.push(VizNode {
            id: node.name.replace('/', "_").replace('.', "_"),
            label,
            node_type: VizNodeType::Task,
            ports: Vec::new(),
            host: None,
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("path".into(), node.path.to_string_lossy().to_string());
                m
            },
        });
    }

    for edge in &call_graph.edges {
        edges.push(VizEdge {
            from_node: edge.from.replace('/', "_").replace('.', "_"),
            from_port: None,
            to_node: edge.to.replace('/', "_").replace('.', "_"),
            to_port: None,
            lane: None,
            mode: Some(format!("call:{}", edge.task_id)),
            message_type: None,
            detach: false,
        });
    }

    let graph = VizGraph {
        name: format!("Call Graph: {}", dir),
        nodes,
        edges,
        subgraphs: Vec::new(),
    };

    let config = VizConfig {
        format: VizFormat::from_str(&args.format)?,
        level: VizLevel::from_str(&args.level)?,
        show_lanes: args.show_lanes,
        show_topology: false,
        show_ports: false,
        show_messages: args.show_messages,
        show_workflows: false,
    };

    let output = vil_viz::render(&graph, &config)?;

    if let Some(path) = &args.output {
        std::fs::write(path, &output).map_err(|e| format!("Write failed: {}", e))?;
        eprintln!("Written to {}", path);
    } else {
        println!("{}", output);
    }

    if args.open {
        let path = args.output.as_deref().unwrap_or("/tmp/vil-call-graph.html");
        if args.output.is_none() {
            std::fs::write(path, &output).map_err(|e| format!("Write failed: {}", e))?;
        }
        open_browser(path);
    }

    Ok(())
}

/// Convert a resolved call target into a VizSubgraph.
fn call_to_subgraph(call_path: &str, resolved: &vil_cli_compile::call_resolver::ResolvedCall) -> VizSubgraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for task in &resolved.workflow.tasks {
        nodes.push(VizNode {
            id: format!(
                "{}_{}",
                call_path.replace('/', "_").replace('.', "_"),
                task.id
            ),
            label: task.name.as_deref().unwrap_or(&task.id).to_string(),
            node_type: VizNodeType::Task,
            ports: Vec::new(),
            host: None,
            metadata: std::collections::HashMap::new(),
        });

        for dep in &task.deps {
            edges.push(VizEdge {
                from_node: format!(
                    "{}_{}",
                    call_path.replace('/', "_").replace('.', "_"),
                    dep.task_id()
                ),
                from_port: None,
                to_node: format!(
                    "{}_{}",
                    call_path.replace('/', "_").replace('.', "_"),
                    task.id
                ),
                to_port: None,
                lane: None,
                mode: None,
                message_type: None,
                detach: dep.is_detached(),
            });
        }
    }

    VizSubgraph {
        parent_node: format!("call:{}", call_path),
        nodes,
        edges,
    }
}
