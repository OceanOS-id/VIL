//! VwfdExecutor — async workflow execution engine for VIL.
//!
//! Walks VilwGraph nodes following edges, evaluates mappings via eval_bridge,
//! dispatches connector calls (async), handles loops/guards/ErrorBoundary.
//!
//! Control flow follows vflow kernel pattern:
//! - Loops: walk full body subgraph per iteration (not just 1 node)
//! - ErrorBoundary: walk body subgraph, catch errors → error edge
//! - Parallel: tokio::join! for actual parallel branches
//! - Gateway: guard condition evaluation with priority

use crate::graph::*;
use crate::eval_bridge;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Execution result.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub output: Value,
    pub variables: HashMap<String, Value>,
    pub steps: u32,
}

/// Execution error.
#[derive(Debug)]
pub struct ExecError {
    pub message: String,
    pub node_id: Option<String>,
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref nid) = self.node_id {
            write!(f, "node '{}': {}", nid, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Async connector dispatch function.
pub type ConnectorFn = Arc<
    dyn Fn(&str, &str, &Value) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
        + Send
        + Sync,
>;

/// Rule evaluation function (sync — CPU-bound).
pub type RuleFn = Box<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

/// Executor configuration.
pub struct ExecConfig {
    pub connector_fn: Option<ConnectorFn>,
    pub rule_fn: Option<RuleFn>,
    pub max_steps: u32,
    pub max_loop_iterations: u32,
    /// Durability store for execution checkpoint/recovery.
    /// If None, execution is stateless (no checkpoint, no recovery).
    pub durability: Option<Arc<crate::DurabilityStore>>,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            connector_fn: None,
            rule_fn: None,
            max_steps: 10_000,
            max_loop_iterations: 1_000,
            durability: None,
        }
    }
}

/// Execute a compiled VilwGraph with input (async).
pub async fn execute(
    graph: &VilwGraph,
    input: Value,
    config: &ExecConfig,
) -> Result<ExecResult, ExecError> {
    let mut vars: HashMap<String, Value> = HashMap::new();

    if let Value::Object(ref map) = input {
        for (k, v) in map {
            vars.insert(k.clone(), v.clone());
        }
    }
    vars.insert("trigger_payload".into(), input.clone());

    // Generate execution ID
    let exec_id = format!("exec_{:016x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    let workflow_id = graph.id.clone();

    // Durability: begin execution
    if let Some(ref store) = config.durability {
        store.begin(&exec_id, &workflow_id, &vars.get("trigger_payload").unwrap_or(&Value::Null));
    }

    let mut steps: u32 = 0;
    let result = walk_subgraph(graph.entry_node, None, graph, &mut vars, config, &mut steps, &exec_id).await;

    // Durability: complete or fail
    if let Some(ref store) = config.durability {
        match &result {
            Ok(_) => store.complete(&exec_id),
            Err(e) => store.fail(&exec_id, &e.to_string()),
        }
    }

    result.map(|output| ExecResult { output, variables: vars, steps })
}

// ── Core walker — walks subgraph until terminal or scope boundary ───────────

/// Walk from `start_idx` executing nodes, following edges.
/// Stops at End/EndTrigger, or when reaching `scope_boundary` (loop back-edge).
/// Returns the output value.
///
/// Uses Box::pin for recursive async (ErrorBoundary, loops call walk_subgraph).
fn walk_subgraph<'a>(
    start_idx: usize,
    scope_boundary: Option<usize>,
    graph: &'a VilwGraph,
    vars: &'a mut HashMap<String, Value>,
    config: &'a ExecConfig,
    steps: &'a mut u32,
    exec_id: &'a str,
) -> Pin<Box<dyn Future<Output = Result<Value, ExecError>> + Send + 'a>> {
    Box::pin(async move {
    let mut current_idx = start_idx;

    loop {
        if *steps >= config.max_steps {
            return Err(ExecError {
                message: format!("exceeded max steps ({})", config.max_steps),
                node_id: None,
            });
        }
        *steps += 1;

        let node = &graph.nodes[current_idx];

        match node.kind {
            NodeKind::Trigger => {}

            NodeKind::Connector => {
                let result = execute_connector(node, vars, config).await?;
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::Transform => {
                let result = execute_transform(node, vars)?;
                tracing::debug!("Transform '{}' output_var={:?} result={}", node.id, node.output_variable, result);
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::VilRules => {
                let result = execute_rules(node, vars, config)?;
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::EndTrigger => {
                return execute_end_trigger(node, vars);
            }

            NodeKind::End => {
                return Ok(vars.get("_last_output").cloned().unwrap_or(Value::Null));
            }

            NodeKind::LoopWhile => {
                execute_loop_while(current_idx, graph, vars, config, steps, exec_id).await?;
                // After loop, advance via _exit edge
                current_idx = find_exit_edge(current_idx, graph)?;
                continue;
            }

            NodeKind::LoopForEach => {
                execute_loop_foreach(current_idx, graph, vars, config, steps, exec_id).await?;
                current_idx = find_exit_edge(current_idx, graph)?;
                continue;
            }

            NodeKind::LoopRepeat => {
                execute_loop_repeat(current_idx, graph, vars, config, steps, exec_id).await?;
                current_idx = find_exit_edge(current_idx, graph)?;
                continue;
            }

            NodeKind::ErrorBoundary => {
                // Walk body subgraph, catch errors → route to error edge
                let normal_edges: Vec<_> = graph.outgoing_edges(current_idx)
                    .iter().filter(|e| e.condition.is_none()).cloned().collect();
                let error_edges: Vec<_> = graph.outgoing_edges(current_idx)
                    .iter().filter(|e| e.condition.as_deref() == Some("_error")).cloned().collect();

                if let Some(normal) = normal_edges.first() {
                    let saved_vars = vars.clone();
                    match walk_subgraph(normal.to_idx, None, graph, vars, config, steps, exec_id).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            *vars = saved_vars;
                            vars.insert("_error".into(), serde_json::json!({
                                "message": e.message,
                                "node_id": e.node_id,
                            }));
                            if let Some(err_edge) = error_edges.first() {
                                current_idx = err_edge.to_idx;
                                continue;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }

            NodeKind::ExclusiveGateway | NodeKind::InclusiveGateway => {
                let next = evaluate_gateway(current_idx, graph, vars, node.kind == NodeKind::InclusiveGateway)?;
                if next.is_empty() {
                    return Err(ExecError {
                        message: "no guard condition matched".into(),
                        node_id: Some(node.id.clone()),
                    });
                }
                current_idx = next[0];
                continue;
            }

            NodeKind::Function => {
                let result = execute_wasm_function(node, vars, config).await?;
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::Sidecar => {
                let result = execute_sidecar(node, vars, config).await?;
                tracing::debug!("Sidecar '{}' output_var={:?} result={}", node.id, node.output_variable, result);
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::SubWorkflow => {
                let result = execute_sub_workflow(node, vars, config).await?;
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::HumanTask => {
                let result = execute_human_task(node, vars, config).await?;
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::NativeCode => {
                let result = execute_native_code(node, vars, config).await?;
                tracing::debug!("NativeCode '{}' output_var={:?} result={}", node.id, node.output_variable, result);
                store_output(node, &result, vars);
                if let Some(ref store) = config.durability {
                    store.checkpoint(exec_id, &node.id, *steps, vars);
                }
            }

            NodeKind::Parallel => {
                // Fork: execute all outgoing branches concurrently
                let edges = graph.outgoing_edges(current_idx);
                if edges.len() > 1 {
                    let join_idx = find_join_for_parallel(current_idx, graph);
                    let branch_starts: Vec<usize> = edges.iter().map(|e| e.to_idx).collect();

                    // Execute branches sequentially but concurrently via join_all
                    // (walk_subgraph borrows &mut vars, so true parallel needs cloning)
                    let mut branch_results: Vec<(Value, HashMap<String, Value>)> = Vec::new();

                    for &branch_start in &branch_starts {
                        let mut branch_vars = vars.clone();
                        let mut branch_steps = 0u32;
                        let result = walk_subgraph(branch_start, join_idx, graph, &mut branch_vars, config, &mut branch_steps, exec_id).await?;
                        *steps += branch_steps;
                        branch_results.push((result, branch_vars));
                    }

                    // Merge all branch results into main vars
                    for (result, branch_vars) in branch_results {
                        for (k, v) in branch_vars {
                            if k != "trigger_payload" && k != "_tenant_id" {
                                vars.insert(k, v);
                            }
                        }
                        vars.insert("_last_output".into(), result);
                    }

                    // Skip to Join node
                    if let Some(ji) = join_idx {
                        current_idx = ji;
                        continue;
                    }
                }
            }

            NodeKind::Join | NodeKind::Noop => {
                // Join: barrier — branches already merged in Parallel handler
                // Noop: passthrough
            }
        }

        // Advance to next node
        current_idx = match find_next_node(current_idx, graph, vars)? {
            Some(next) => {
                // Check scope boundary (loop back-edge)
                if let Some(boundary) = scope_boundary {
                    if next == boundary {
                        return Ok(vars.get("_last_output").cloned().unwrap_or(Value::Null));
                    }
                }
                next
            }
            None => {
                return Ok(vars.get("_last_output").cloned().unwrap_or(Value::Null));
            }
        };
    }
    }) // close Box::pin(async move { ... })
}

fn store_output(node: &VilwNode, result: &Value, vars: &mut HashMap<String, Value>) {
    if let Some(ref out_var) = node.output_variable {
        vars.insert(out_var.clone(), result.clone());
    }
    vars.insert("_last_output".into(), result.clone());
}

// ── Loops — walk full body subgraph per iteration ───────────────────────────

async fn execute_loop_while(
    loop_idx: usize,
    graph: &VilwGraph,
    vars: &mut HashMap<String, Value>,
    config: &ExecConfig,
    steps: &mut u32,
    exec_id: &str,
) -> Result<(), ExecError> {
    let node = &graph.nodes[loop_idx];
    let condition = node.config.get("condition")
        .and_then(|v| v.as_str()).unwrap_or("false");
    let max_iter = node.config.get("max_iterations")
        .and_then(|v| v.as_u64()).unwrap_or(config.max_loop_iterations as u64) as u32;

    let body_idx = find_body_edge(loop_idx, graph);

    let mut iteration = 0u32;
    while iteration < max_iter {
        match vil_expr::evaluate_bool(condition, vars) {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => return Err(ExecError {
                message: format!("loop condition: {}", e),
                node_id: Some(node.id.clone()),
            }),
        }
        vars.insert("_loop_index".into(), Value::Number(iteration.into()));
        iteration += 1;

        if let Some(bidx) = body_idx {
            // Walk FULL body subgraph — stop when edge points back to loop node
            walk_subgraph(bidx, Some(loop_idx), graph, vars, config, steps, exec_id).await?;
        }
    }

    Ok(())
}

async fn execute_loop_foreach(
    loop_idx: usize,
    graph: &VilwGraph,
    vars: &mut HashMap<String, Value>,
    config: &ExecConfig,
    steps: &mut u32,
    exec_id: &str,
) -> Result<(), ExecError> {
    let node = &graph.nodes[loop_idx];
    let collection_expr = node.config.get("collection")
        .and_then(|v| v.as_str()).unwrap_or("[]");
    let item_var = node.config.get("item_variable")
        .and_then(|v| v.as_str()).unwrap_or("_item");

    let collection = vil_expr::evaluate(collection_expr, vars)
        .map_err(|e| ExecError {
            message: format!("foreach collection: {}", e),
            node_id: Some(node.id.clone()),
        })?;

    let items = match &collection {
        Value::Array(arr) => arr.clone(),
        _ => Vec::new(),
    };

    let body_idx = find_body_edge(loop_idx, graph);

    for (i, item) in items.iter().enumerate() {
        vars.insert(item_var.into(), item.clone());
        vars.insert("_loop_index".into(), Value::Number(i.into()));

        if let Some(bidx) = body_idx {
            walk_subgraph(bidx, Some(loop_idx), graph, vars, config, steps, exec_id).await?;
        }
    }

    Ok(())
}

async fn execute_loop_repeat(
    loop_idx: usize,
    graph: &VilwGraph,
    vars: &mut HashMap<String, Value>,
    config: &ExecConfig,
    steps: &mut u32,
    exec_id: &str,
) -> Result<(), ExecError> {
    let node = &graph.nodes[loop_idx];
    let count = node.config.get("repeat_count")
        .and_then(|v| v.as_u64()).unwrap_or(1) as u32;

    let body_idx = find_body_edge(loop_idx, graph);

    for i in 0..count {
        vars.insert("_loop_index".into(), Value::Number(i.into()));

        if let Some(bidx) = body_idx {
            walk_subgraph(bidx, Some(loop_idx), graph, vars, config, steps, exec_id).await?;
        }
    }

    Ok(())
}

/// Find body edge (non-exit, non-error) from loop node.
fn find_body_edge(loop_idx: usize, graph: &VilwGraph) -> Option<usize> {
    graph.outgoing_edges(loop_idx)
        .iter()
        .find(|e| e.condition.is_none() || e.condition.as_deref() != Some("_exit"))
        .map(|e| e.to_idx)
}

/// Find exit edge from loop node.
fn find_exit_edge(loop_idx: usize, graph: &VilwGraph) -> Result<usize, ExecError> {
    let edges = graph.outgoing_edges(loop_idx);
    // Prefer _exit edge
    if let Some(exit) = edges.iter().find(|e| e.condition.as_deref() == Some("_exit")) {
        return Ok(exit.to_idx);
    }
    // Fallback: last edge or next node
    edges.last().map(|e| e.to_idx).ok_or(ExecError {
        message: "loop has no exit edge".into(),
        node_id: Some(graph.nodes[loop_idx].id.clone()),
    })
}

/// Find the Join node that corresponds to a Parallel fork.
/// Walks outgoing edges from Parallel → follows first branch → looks for Join node.
fn find_join_for_parallel(_parallel_idx: usize, graph: &VilwGraph) -> Option<usize> {
    // Simple heuristic: scan all nodes for a Join that has edges coming from
    // branches that start at this Parallel.
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.kind == NodeKind::Join {
            // Check if any branch path from this parallel leads to this join
            let incoming = graph.edges.iter().filter(|e| e.to_idx == i).count();
            if incoming >= 2 {
                return Some(i);
            }
        }
    }
    None
}

// ── Node executors ──────────────────────────────────────────────────────────

async fn execute_connector(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let input = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;

    let connector_ref = node.config.get("connector_ref")
        .and_then(|v| v.as_str()).unwrap_or("");
    let operation = node.config.get("operation")
        .and_then(|v| v.as_str()).unwrap_or("");

    let mut input_value = serde_json::to_value(&input).unwrap_or(Value::Null);

    // Inject streaming metadata from connector_config into input
    // so registry dispatch can decide SSE vs buffered
    if let Value::Object(ref mut map) = input_value {
        if let Some(streaming) = node.config.get("streaming") {
            map.insert("_streaming".into(), streaming.clone());
        }
        if let Some(dialect) = node.config.get("dialect") {
            map.insert("_dialect".into(), dialect.clone());
        }
        if let Some(tap) = node.config.get("json_tap") {
            map.insert("_json_tap".into(), tap.clone());
        }
        if let Some(fmt) = node.config.get("stream_format") {
            map.insert("_stream_format".into(), fmt.clone());
        }
    }

    if let Some(ref connector_fn) = config.connector_fn {
        connector_fn(connector_ref, operation, &input_value)
            .await
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({
            "_stub": true,
            "connector_ref": connector_ref,
            "operation": operation,
            "input": input_value,
        }))
    }
}

fn execute_transform(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
) -> Result<Value, ExecError> {
    let result = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;

    // Unwrap single-mapping transforms: if there's exactly one mapping and its
    // target matches the output_variable, return the value directly instead of
    // wrapping it as {"target_name": value}. This prevents double nesting when
    // downstream nodes reference output_variable.field.
    if node.mappings.len() == 1 {
        if let Some(ref out_var) = node.output_variable {
            if node.mappings[0].target == *out_var {
                if let Some(val) = result.get(out_var) {
                    return Ok(val.clone());
                }
            }
        }
    }

    Ok(serde_json::to_value(&result).unwrap_or(Value::Null))
}

fn execute_rules(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let rule_set_id = node.config.get("rule_set_id")
        .and_then(|v| v.as_str()).unwrap_or("");

    let input = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;
    let input_value = serde_json::to_value(&input).unwrap_or(Value::Null);

    if let Some(ref rule_fn) = config.rule_fn {
        rule_fn(rule_set_id, &input_value)
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({
            "_stub": true, "_rule": rule_set_id,
        }))
    }
}

fn execute_end_trigger(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
) -> Result<Value, ExecError> {
    if let Some(fr) = node.config.get("final_response") {
        let lang = fr.get("language").and_then(|v| v.as_str()).unwrap_or("vil-expr");
        let source = fr.get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("_last_output");

        match lang {
            "vil-expr" | "cel" => {
                vil_expr::evaluate(source, vars)
                    .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
            }
            "literal" => Ok(Value::String(source.to_string())),
            "spv1" => Ok(Value::String(crate::spv1::eval_template(source, vars))),
            _ => Ok(vars.get("_last_output").cloned().unwrap_or(Value::Null)),
        }
    } else {
        Ok(vars.get("_last_output").cloned().unwrap_or(Value::Null))
    }
}

// ── WASM Function ───────────────────────────────────────────────────────────

async fn execute_wasm_function(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let module_ref = node.config.get("module_ref").and_then(|v| v.as_str()).unwrap_or("");
    let function_name = node.config.get("function_name").and_then(|v| v.as_str()).unwrap_or("execute");

    let input = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;
    let mut input_value = serde_json::to_value(&input).unwrap_or(Value::Null);

    // Inject WASM metadata for registry dispatch
    if let Value::Object(ref mut map) = input_value {
        map.insert("_wasm_module".into(), Value::String(module_ref.into()));
        map.insert("_wasm_function".into(), Value::String(function_name.into()));
    }

    // Dispatch via connector_fn with vastar.wasm.{module} ref
    let connector_ref = format!("vastar.wasm.{}", module_ref);
    if let Some(ref connector_fn) = config.connector_fn {
        connector_fn(&connector_ref, function_name, &input_value)
            .await
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({"_stub": true, "_wasm": module_ref, "_function": function_name}))
    }
}

// ── Sidecar ─────────────────────────────────────────────────────────────────

async fn execute_sidecar(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let target = node.config.get("target").and_then(|v| v.as_str()).unwrap_or("");
    let method = node.config.get("method").and_then(|v| v.as_str()).unwrap_or("execute");

    let input = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;
    let mut input_value = serde_json::to_value(&input).unwrap_or(Value::Null);

    if let Value::Object(ref mut map) = input_value {
        map.insert("_sidecar_target".into(), Value::String(target.into()));
        map.insert("_sidecar_method".into(), Value::String(method.into()));
    }

    let connector_ref = format!("vastar.sidecar.{}", target);
    if let Some(ref connector_fn) = config.connector_fn {
        connector_fn(&connector_ref, method, &input_value)
            .await
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({"_stub": true, "_sidecar": target, "_method": method}))
    }
}

// ── SubWorkflow ─────────────────────────────────────────────────────────────

async fn execute_sub_workflow(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let workflow_ref = node.config.get("workflow_ref").and_then(|v| v.as_str()).unwrap_or("");

    // Build sub-workflow input from mappings or pass all vars
    let input = if !node.mappings.is_empty() {
        let mapped = eval_bridge::eval_all_mappings(&node.mappings, vars)
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;
        serde_json::to_value(&mapped).unwrap_or(Value::Null)
    } else {
        serde_json::to_value(vars).unwrap_or(Value::Null)
    };

    // Dispatch as connector call — handler layer resolves workflow_ref → graph
    let connector_ref = format!("vastar.workflow.{}", workflow_ref);
    if let Some(ref connector_fn) = config.connector_fn {
        connector_fn(&connector_ref, "execute", &input)
            .await
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({"_stub": true, "_sub_workflow": workflow_ref}))
    }
}

// ── HumanTask ───────────────────────────────────────────────────────────────

async fn execute_human_task(
    node: &VilwNode,
    _vars: &HashMap<String, Value>,
    _config: &ExecConfig,
) -> Result<Value, ExecError> {
    let task_type = node.config.get("task_type").and_then(|v| v.as_str()).unwrap_or("approval");
    let assignee = node.config.get("assignee").and_then(|v| v.as_str());

    // HumanTask requires external task management system.
    // In VIL free tier: return stub. In vflow: parks token until task completed.
    Ok(serde_json::json!({
        "_human_task": true,
        "task_type": task_type,
        "assignee": assignee,
        "_note": "HumanTask requires external task manager. Auto-approved in VIL free tier.",
        "approved": true,
    }))
}

// ── NativeCode ──────────────────────────────────────────────────────────────

async fn execute_native_code(
    node: &VilwNode,
    vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Result<Value, ExecError> {
    let handler_ref = node.config.get("handler_ref").and_then(|v| v.as_str()).unwrap_or("");

    let input = eval_bridge::eval_all_mappings(&node.mappings, vars)
        .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })?;
    let input_value = serde_json::to_value(&input).unwrap_or(Value::Null);

    // Dispatch via ConnectorFn with vastar.code.{handler_ref}
    let connector_ref = format!("vastar.code.{}", handler_ref);
    if let Some(ref connector_fn) = config.connector_fn {
        connector_fn(&connector_ref, "execute", &input_value)
            .await
            .map_err(|e| ExecError { message: e, node_id: Some(node.id.clone()) })
    } else {
        Ok(serde_json::json!({"_stub": true, "_native_code": handler_ref}))
    }
}

// ── Flow navigation ─────────────────────────────────────────────────────────

fn find_next_node(
    current_idx: usize,
    graph: &VilwGraph,
    vars: &HashMap<String, Value>,
) -> Result<Option<usize>, ExecError> {
    let mut edges: Vec<_> = graph.outgoing_edges(current_idx);
    edges.sort_by(|a, b| b.priority.cmp(&a.priority));

    for edge in &edges {
        if edge.detached { continue; }
        if let Some(ref cond) = edge.condition {
            if cond == "_error" || cond == "_exit" { continue; }
            match vil_expr::evaluate_bool(cond, vars) {
                Ok(true) => return Ok(Some(edge.to_idx)),
                Ok(false) => continue,
                Err(e) => return Err(ExecError {
                    message: format!("guard eval: {}", e), node_id: None,
                }),
            }
        } else {
            return Ok(Some(edge.to_idx));
        }
    }

    Ok(None)
}

fn evaluate_gateway(
    node_idx: usize,
    graph: &VilwGraph,
    vars: &HashMap<String, Value>,
    inclusive: bool,
) -> Result<Vec<usize>, ExecError> {
    let mut edges: Vec<_> = graph.outgoing_edges(node_idx);
    edges.sort_by(|a, b| b.priority.cmp(&a.priority));

    let mut matched = Vec::new();
    for edge in &edges {
        if let Some(ref cond) = edge.condition {
            match vil_expr::evaluate_bool(cond, vars) {
                Ok(true) => {
                    matched.push(edge.to_idx);
                    if !inclusive { break; }
                }
                Ok(false) => {}
                Err(e) => return Err(ExecError {
                    message: format!("guard eval: {}", e), node_id: None,
                }),
            }
        } else {
            matched.push(edge.to_idx);
            if !inclusive { break; }
        }
    }

    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler;
    use serde_json::json;

    const SIMPLE_WF: &str = r#"
version: "3.0"
metadata:
  id: test-exec
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /test }
        response_mode: buffered
        end_activity: respond
      output_variable: trigger_payload
    - id: step1
      activity_type: Connector
      connector_config:
        connector_ref: vastar.http
        operation: post
      input_mappings:
        - target: url
          source: { language: literal, source: "http://example.com" }
        - target: body
          source: { language: vil-expr, source: '{"name": trigger_payload.name}' }
      output_variable: step1_result
    - id: respond
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response:
          language: vil-expr
          source: '{"result": step1_result, "input_name": trigger_payload.name}'
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: step1 } }
    - { id: f2, from: { node: step1 }, to: { node: respond } }
    - { id: f3, from: { node: respond }, to: { node: end } }
  variables:
    - { name: trigger_payload, type: object }
    - { name: step1_result, type: object }
"#;

    #[tokio::test]
    async fn test_execute_simple_stub() {
        let graph = compiler::compile(SIMPLE_WF).unwrap();
        let input = json!({"name": "Alice"});
        let config = ExecConfig::default();
        let result = execute(&graph, input, &config).await.unwrap();
        assert_eq!(result.output["input_name"], "Alice");
        assert!(result.output["result"]["_stub"].as_bool().unwrap_or(false));
        assert_eq!(result.steps, 3);
    }

    #[tokio::test]
    async fn test_execute_with_connector_fn() {
        let graph = compiler::compile(SIMPLE_WF).unwrap();
        let input = json!({"name": "Bob"});
        let config = ExecConfig {
            connector_fn: Some(Arc::new(|_ref, _op, input| {
                let input = input.clone();
                Box::pin(async move {
                    Ok(json!({"status": "ok", "echo": input}))
                })
            })),
            ..Default::default()
        };
        let result = execute(&graph, input, &config).await.unwrap();
        assert_eq!(result.output["input_name"], "Bob");
        assert_eq!(result.output["result"]["status"], "ok");
    }

    const GUARD_WF: &str = r#"
version: "3.0"
metadata:
  id: test-guard
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /guard }
        response_mode: buffered
        end_activity: high-resp
      output_variable: trigger_payload
    - id: gateway
      activity_type: ExclusiveGateway
    - id: high
      activity_type: Connector
      connector_config: { connector_ref: vastar.http, operation: post }
      output_variable: high_result
    - id: low
      activity_type: Connector
      connector_config: { connector_ref: vastar.http, operation: post }
      output_variable: low_result
    - id: high-resp
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response: { language: vil-expr, source: '{"route": "high"}' }
    - id: low-resp
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response: { language: vil-expr, source: '{"route": "low"}' }
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: gateway } }
    - { id: f2, from: { node: gateway }, to: { node: high }, condition: "trigger_payload.score > 80", priority: 1 }
    - { id: f3, from: { node: gateway }, to: { node: low }, condition: "trigger_payload.score <= 80", priority: 0 }
    - { id: f4, from: { node: high }, to: { node: high-resp } }
    - { id: f5, from: { node: low }, to: { node: low-resp } }
    - { id: f6, from: { node: high-resp }, to: { node: end } }
    - { id: f7, from: { node: low-resp }, to: { node: end } }
"#;

    #[tokio::test]
    async fn test_guard_high() {
        let graph = compiler::compile(GUARD_WF).unwrap();
        let result = execute(&graph, json!({"score": 90}), &ExecConfig::default()).await.unwrap();
        assert_eq!(result.output["route"], "high");
    }

    #[tokio::test]
    async fn test_guard_low() {
        let graph = compiler::compile(GUARD_WF).unwrap();
        let result = execute(&graph, json!({"score": 50}), &ExecConfig::default()).await.unwrap();
        assert_eq!(result.output["route"], "low");
    }
}
