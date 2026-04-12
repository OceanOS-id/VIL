//! MCP tool implementations.

use serde_json::Value;
use crate::cli;

/// Dispatch tool call by name.
pub fn call_tool(name: &str, arguments: &Value) -> Value {
    match name {
        "vil_compile" => tool_compile(arguments),
        "vil_lint" => tool_lint(arguments),
        "vil_list" => tool_list(arguments),
        "vil_explain" => tool_explain(arguments),
        "vil_scaffold_workflow" => tool_scaffold(arguments),
        _ => serde_json::json!({
            "content": [{"type": "text", "text": format!("unknown tool: {}", name)}],
            "isError": true
        }),
    }
}

fn tool_compile(args: &Value) -> Value {
    let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("");
    if file.is_empty() {
        return mcp_error("file parameter required");
    }

    match cli::compile_vwfd(file) {
        Ok(result) => mcp_text(format!(
            "Compiled: {} (id={}, {} nodes, route={:?}, {}B, {}ms)",
            file, result.id, result.node_count, result.route, result.bytes, result.duration_ms
        )),
        Err(e) => mcp_error(format!("Compile error: {}", e)),
    }
}

fn tool_lint(args: &Value) -> Value {
    let file = args.get("file").and_then(|v| v.as_str());
    let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);

    if all {
        let dir = args.get("dir").and_then(|v| v.as_str()).unwrap_or("workflows");
        let results = cli::lint_dir(dir);
        let mut output = String::new();
        for r in &results {
            output.push_str(&format_lint_result(r));
        }
        if output.is_empty() { output = "No VWFD files found.".into(); }
        mcp_text(output)
    } else if let Some(f) = file {
        let result = cli::lint_vwfd(f);
        mcp_text(format_lint_result(&result))
    } else {
        mcp_error("file or all=true required")
    }
}

fn format_lint_result(r: &cli::LintResult) -> String {
    let mut out = format!("=== {} ===\n", r.file);
    for e in &r.errors { out.push_str(&format!("  ERROR [{}] {}", e.code, e.message)); if let Some(ref l) = e.location { out.push_str(&format!(" ({})", l)); } out.push('\n'); }
    for w in &r.warnings { out.push_str(&format!("  WARN  [{}] {}", w.code, w.message)); if let Some(ref l) = w.location { out.push_str(&format!(" ({})", l)); } out.push('\n'); }
    for i in &r.infos { out.push_str(&format!("  INFO  [{}] {}", i.code, i.message)); if let Some(ref l) = i.location { out.push_str(&format!(" ({})", l)); } out.push('\n'); }
    if r.errors.is_empty() && r.warnings.is_empty() && r.infos.is_empty() {
        out.push_str("  OK — no issues\n");
    }
    out
}

fn tool_list(args: &Value) -> Value {
    let dir = args.get("dir").and_then(|v| v.as_str()).unwrap_or("workflows");
    let result = crate::loader::load_dir(dir);

    let mut workflows: Vec<Value> = result.graphs.iter().map(|g| {
        serde_json::json!({
            "id": g.id,
            "route": g.webhook_route,
            "trigger": g.trigger_type,
            "nodes": g.node_count(),
        })
    }).collect();

    for e in &result.errors {
        workflows.push(serde_json::json!({"file": e.file, "error": e.error}));
    }

    mcp_text(serde_json::to_string_pretty(&workflows).unwrap_or_default())
}

fn tool_explain(args: &Value) -> Value {
    let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
    let explanation = match code {
        "VIL-L001" => "External connector (http, mq, storage) should have retry_policy for resilience.\nAdd: retry_policy: { max_attempts: 3, base_delay_ms: 1000 }",
        "VIL-L003" => "connector_ref should start with 'vastar.' namespace.\nExample: vastar.http, vastar.db.postgres, vastar.mq.nats",
        "VIL-L004" => "Connector should specify timeout_ms. Default is 30s which may be too long.\nAdd: timeout_ms: 10000",
        "VIL-L005" => "output_variable is defined but not referenced in any downstream mapping or response.\nEither use the variable or remove the output_variable declaration.",
        "VIL-L006" => "A flow edge references a node that doesn't exist.\nCheck from/to node IDs match activity/control IDs.",
        "VIL-L007" => "EndTrigger exists but the Trigger activity has no end_activity pointing to it.\nAdd: end_activity: <end_trigger_id> to trigger_config.",
        "VIL-L008" => "Workflow has no durability config. Defaults to 'eventual'.\nAdd: durability: { enabled: true, default_mode: eventual }",
        "VIL-L009" => "Mutating connector (post/put/delete/insert/update) without compensation.\nSaga rollback won't be possible if this step fails. Add compensation config.",
        _ => "Unknown lint code. Valid codes: VIL-L001 through VIL-L009.",
    };
    mcp_text(format!("[{}]\n{}", code, explanation))
}

fn tool_scaffold(args: &Value) -> Value {
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("new-workflow");
    let route = args.get("route").and_then(|v| v.as_str()).unwrap_or("/api/new");
    let connectors: Vec<&str> = args.get("connectors")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut yaml = format!(r#"version: "3.0"
metadata:
  id: {name}
  name: "{name}"
spec:
  durability:
    enabled: true
    default_mode: eventual

  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        route: {route}
        response_mode: buffered
        end_activity: respond
      output_variable: trigger_payload
"#);

    for (i, cref) in connectors.iter().enumerate() {
        let step_id = format!("step_{}", i + 1);
        yaml.push_str(&format!(r#"
    - id: {step_id}
      activity_type: Connector
      connector_config:
        connector_ref: {cref}
        operation: post
        timeout_ms: 10000
        retry_policy:
          max_attempts: 3
          base_delay_ms: 1000
      input_mappings:
        - target: body
          source:
            language: v-cel
            source: 'trigger_payload'
      output_variable: {step_id}_result
"#));
    }

    yaml.push_str(r#"
    - id: respond
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response:
          language: v-cel
          source: '{"status": "ok"}'

    - id: end
      activity_type: End

  flows:
"#);

    // Build flow chain
    let mut flow_nodes = vec!["trigger".to_string()];
    for (i, _) in connectors.iter().enumerate() {
        flow_nodes.push(format!("step_{}", i + 1));
    }
    flow_nodes.push("respond".into());
    flow_nodes.push("end".into());

    for (i, pair) in flow_nodes.windows(2).enumerate() {
        yaml.push_str(&format!("    - {{ id: f{}, from: {{ node: {} }}, to: {{ node: {} }} }}\n",
            i + 1, pair[0], pair[1]));
    }

    mcp_text(yaml)
}

// ── MCP response helpers ──

fn mcp_text(text: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": text.into()}]
    })
}

fn mcp_error(msg: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{"type": "text", "text": msg.into()}],
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_explain() {
        let result = call_tool("vil_explain", &json!({"code": "VIL-L001"}));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("retry_policy"));
    }

    #[test]
    fn test_tool_scaffold() {
        let result = call_tool("vil_scaffold_workflow", &json!({
            "name": "order-api",
            "route": "/api/orders",
            "connectors": ["vastar.http", "vastar.db.postgres"]
        }));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("order-api"));
        assert!(text.contains("/api/orders"));
        assert!(text.contains("vastar.http"));
        assert!(text.contains("vastar.db.postgres"));
        assert!(text.contains("step_1"));
        assert!(text.contains("step_2"));
    }

    #[test]
    fn test_tool_unknown() {
        let result = call_tool("nonexistent", &json!({}));
        assert!(result["isError"].as_bool().unwrap_or(false));
    }
}
