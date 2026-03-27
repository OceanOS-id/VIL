// =============================================================================
// VIL YAML Pipeline Definitions
// =============================================================================
// Parse and run pipeline definitions from YAML files.
//
// Example YAML:
// ```yaml
// name: ai-gateway
// nodes:
//   webhook:
//     type: http-sink
//     port: 3080
//     path: /trigger
//   inference:
//     type: http-source
//     url: http://localhost:18081/api/v1/credits/stream
//     format: sse
//     json_tap: "choices[0].delta.content"
// routes:
//   - from: webhook.trigger_out
//     to: inference.trigger_in
//     mode: LoanWrite
//   - from: inference.data_out
//     to: webhook.data_in
//     mode: LoanWrite
//   - from: inference.ctrl_out
//     to: webhook.ctrl_in
//     mode: Copy
// ```
// =============================================================================

use anyhow::{Context, Result};
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PipelineConfig {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    pub nodes: HashMap<String, NodeConfig>,
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub observability: Option<ObservabilityConfig>,
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub json_tap: Option<String>,
    #[serde(default)]
    pub post_body: Option<serde_json::Value>,
    /// Execution mode: "native" (default), "wasm", "sidecar"
    #[serde(default)]
    pub exec: Option<String>,
    /// WASM module path (when exec: wasm)
    #[serde(default)]
    pub module: Option<String>,
    /// WASM function name (when exec: wasm)
    #[serde(default)]
    pub function: Option<String>,
    /// WASM pool size (when exec: wasm)
    #[serde(default)]
    pub pool_size: Option<usize>,
    /// Sidecar command (when exec: sidecar)
    #[serde(default)]
    pub command: Option<String>,
    /// Sidecar script path (when exec: sidecar)
    #[serde(default)]
    pub script: Option<String>,
    /// Sidecar method to invoke
    #[serde(default)]
    pub method: Option<String>,
}

fn default_port() -> u16 {
    3080
}
fn default_path() -> String {
    "/trigger".to_string()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RouteConfig {
    pub from: String,
    pub to: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "LoanWrite".to_string()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ObservabilityConfig {
    #[serde(default)]
    pub prometheus: Option<PrometheusConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PrometheusConfig {
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

fn default_metrics_port() -> u16 {
    9090
}

/// Parse a YAML pipeline file and run it.
/// Supports N sinks, N sources — any DAG topology.
pub fn run_yaml_pipeline(path: &str, port_override: Option<u16>) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read pipeline file: {}", path))?;

    let config: PipelineConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML pipeline: {}", path))?;

    println!("{} Pipeline: {}", "Loading".green().bold(), config.name);
    println!("  Nodes: {}", config.nodes.len());
    println!("  Routes: {}", config.routes.len());

    // Classify nodes
    let mut sinks: Vec<(&String, &NodeConfig)> = Vec::new();
    let mut sources: Vec<(&String, &NodeConfig)> = Vec::new();
    let mut transforms: Vec<(&String, &NodeConfig)> = Vec::new();

    for (name, node) in &config.nodes {
        match node.node_type.as_str() {
            "http-sink" => sinks.push((name, node)),
            "http-source" => sources.push((name, node)),
            "transform" => transforms.push((name, node)),
            other => {
                println!(
                    "  {} Unknown node type: {} ({})",
                    "Warning:".yellow(),
                    other,
                    name
                );
            }
        }
    }

    if sinks.is_empty() {
        anyhow::bail!("Pipeline must have at least one http-sink node");
    }
    if sources.is_empty() {
        anyhow::bail!("Pipeline must have at least one http-source node");
    }

    // Init SHM runtime
    let world = std::sync::Arc::new(
        vil_rt::VastarRuntimeWorld::new_shared()
            .map_err(|e| anyhow::anyhow!("Failed to init VIL SHM runtime: {}", e))?,
    );

    // Build and register all sink nodes
    let mut sink_builders: Vec<(String, vil_sdk::http::HttpSinkBuilder)> = Vec::new();
    let mut sink_handles: HashMap<String, vil_rt::ProcessHandle> = HashMap::new();

    for (name, node) in &sinks {
        let port = if sinks.len() == 1 {
            port_override.unwrap_or(node.port)
        } else {
            node.port
        };
        let builder = vil_sdk::http::HttpSinkBuilder::new(name.as_str())
            .port(port)
            .path(&node.path)
            .out_port("trigger_out")
            .in_port("response_data_in")
            .ctrl_in_port("response_ctrl_in");

        let handle = world
            .register_process(builder.build_spec())
            .map_err(|e| anyhow::anyhow!("Failed to register sink '{}': {}", name, e))?;

        println!(
            "  {} http-sink '{}' on http://localhost:{}{}",
            "->".green(),
            name,
            port,
            node.path
        );

        sink_handles.insert(name.to_string(), handle);
        sink_builders.push((name.to_string(), builder));
    }

    // Build and register all source nodes
    let mut source_builders: Vec<(String, vil_sdk::http::HttpSourceBuilder)> = Vec::new();
    let mut source_handles: HashMap<String, vil_rt::ProcessHandle> = HashMap::new();

    for (name, node) in &sources {
        let upstream_url = node
            .url
            .as_deref()
            .with_context(|| format!("http-source '{}' must have a 'url' field", name))?;

        let format = match node.format.as_deref() {
            Some("sse") | Some("SSE") => vil_sdk::http::HttpFormat::SSE,
            Some("ndjson") | Some("NDJSON") => vil_sdk::http::HttpFormat::NDJSON,
            _ => vil_sdk::http::HttpFormat::Raw,
        };

        let json_tap = node
            .json_tap
            .as_deref()
            .unwrap_or("choices[0].delta.content");

        let mut builder = vil_sdk::http::HttpSourceBuilder::new(name.as_str())
            .url(upstream_url)
            .format(format)
            .json_tap(json_tap)
            .in_port("trigger_in")
            .out_port("response_data_out")
            .ctrl_out_port("response_ctrl_out");

        if let Some(body) = &node.post_body {
            builder = builder.post_json(body.clone());
        }

        let handle = world
            .register_process(builder.build_spec())
            .map_err(|e| anyhow::anyhow!("Failed to register source '{}': {}", name, e))?;

        println!(
            "  {} http-source '{}' -> {}",
            "->".green(),
            name,
            upstream_url
        );

        source_handles.insert(name.to_string(), handle);
        source_builders.push((name.to_string(), builder));
    }

    // Build and register all transform nodes
    let mut transform_builders: Vec<(String, crate::transform_builder::TransformBuilder)> =
        Vec::new();
    let mut transform_handles: HashMap<String, vil_rt::ProcessHandle> = HashMap::new();

    for (name, _node) in &transforms {
        let builder = crate::transform_builder::TransformBuilder::new(name.as_str());
        // Uses default ports: "in" (Data/In) + "out" (Data/Out)

        let handle = world
            .register_process(builder.build_spec())
            .map_err(|e| anyhow::anyhow!("Failed to register transform '{}': {}", name, e))?;

        println!("  {} transform '{}'", "->".green(), name);

        transform_handles.insert(name.to_string(), handle);
        transform_builders.push((name.to_string(), builder));
    }

    // Wire routes
    println!();
    let all_handles: HashMap<&str, &vil_rt::ProcessHandle> = sink_handles
        .iter()
        .chain(source_handles.iter())
        .chain(transform_handles.iter())
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    for route in &config.routes {
        let from_parts: Vec<&str> = route.from.splitn(2, '.').collect();
        let to_parts: Vec<&str> = route.to.splitn(2, '.').collect();

        if from_parts.len() != 2 || to_parts.len() != 2 {
            println!(
                "  {} Skipping malformed route: {} -> {}",
                "Warning:".yellow(),
                route.from,
                route.to
            );
            continue;
        }

        let (from_node, from_port) = (from_parts[0], from_parts[1]);
        let (to_node, to_port) = (to_parts[0], to_parts[1]);

        let from_handle = match all_handles.get(from_node) {
            Some(h) => h,
            None => {
                println!(
                    "  {} Route skipped: node '{}' not registered (transform?)",
                    "Note:".cyan(),
                    from_node
                );
                continue;
            }
        };

        let to_handle = match all_handles.get(to_node) {
            Some(h) => h,
            None => {
                println!(
                    "  {} Route skipped: node '{}' not registered (transform?)",
                    "Note:".cyan(),
                    to_node
                );
                continue;
            }
        };

        match (from_handle.port_id(from_port), to_handle.port_id(to_port)) {
            (Ok(from_pid), Ok(to_pid)) => {
                world.connect(from_pid, to_pid);
                println!(
                    "  {} {}.{} → {}.{}",
                    "WIRE".green(),
                    from_node,
                    from_port,
                    to_node,
                    to_port
                );
            }
            (Err(e), _) => {
                println!(
                    "  {} Port '{}' not found on '{}': {}",
                    "Warning:".yellow(),
                    from_port,
                    from_node,
                    e
                );
            }
            (_, Err(e)) => {
                println!(
                    "  {} Port '{}' not found on '{}': {}",
                    "Warning:".yellow(),
                    to_port,
                    to_node,
                    e
                );
            }
        }
    }

    // Print test curl
    if let Some((name, node)) = sinks.first() {
        let port = if sinks.len() == 1 {
            port_override.unwrap_or(node.port)
        } else {
            node.port
        };
        println!("Test with:");
        println!("  curl -N -X POST -H 'Content-Type: application/json' \\");
        println!(
            "    -d '{{\"prompt\": \"hello\"}}' http://localhost:{}{}\n",
            port, node.path
        );
    }

    // Spawn all workers
    let mut threads: Vec<std::thread::JoinHandle<()>> = Vec::new();

    for (name, builder) in sink_builders {
        let handle = sink_handles.remove(&name).expect("handle missing");
        let sink_http = vil_sdk::http::HttpSink::from_builder(builder);
        let w = world.clone();
        threads.push(sink_http.run_worker::<vil_types::GenericToken>(w, handle));
    }

    for (name, builder) in source_builders {
        let handle = source_handles.remove(&name).expect("handle missing");
        let source_http = vil_sdk::http::HttpSource::from_builder(builder);
        let w = world.clone();
        threads.push(source_http.run_worker::<vil_types::GenericToken>(w, handle));
    }

    for (name, builder) in transform_builders {
        let handle = transform_handles.remove(&name).expect("handle missing");
        let transform_node = crate::transform_builder::TransformNode::from_builder(builder);
        let w = world.clone();
        // Passthrough transform — data goes in, same data comes out
        threads.push(transform_node.run_worker(w, handle, |input| input.to_vec()));
    }

    println!(
        "{} Running {} worker(s)...",
        "OK".green().bold(),
        threads.len()
    );

    for t in threads {
        t.join().expect("Worker panicked");
    }

    Ok(())
}

/// Validate a YAML pipeline file without running it.
pub fn validate_yaml_pipeline(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read pipeline file: {}", path))?;

    let config: PipelineConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML: {}", path))?;

    println!(
        "{} Pipeline '{}' is valid",
        "OK".green().bold(),
        config.name
    );

    // Validate node references in routes
    for route in &config.routes {
        let from_node = route.from.split('.').next().unwrap_or("");
        let to_node = route.to.split('.').next().unwrap_or("");

        if !config.nodes.contains_key(from_node) {
            println!(
                "  {} Route references unknown node: '{}'",
                "Warning:".yellow(),
                from_node
            );
        }
        if !config.nodes.contains_key(to_node) {
            println!(
                "  {} Route references unknown node: '{}'",
                "Warning:".yellow(),
                to_node
            );
        }
    }

    println!("  Nodes: {}", config.nodes.len());
    println!("  Routes: {}", config.routes.len());

    Ok(())
}
