//! VIL VWFD Compiler — VWFD YAML → VilwGraph.
//!
//! Validates expressions, compiles vil_query to SQL, rejects unsupported features.

use crate::spec::*;
use crate::graph::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub location: Option<String>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref loc) = self.location {
            write!(f, "{}: {}", loc, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Compile VWFD YAML string → VilwGraph.
pub fn compile(yaml: &str) -> Result<VilwGraph, CompileError> {
    // 1. Parse YAML
    let doc: VwfdDocument = serde_yaml::from_str(yaml)
        .map_err(|e| CompileError { message: format!("YAML parse: {}", e), location: None })?;

    // 2. Build node index
    let mut node_map: HashMap<String, usize> = HashMap::new();
    let mut nodes = Vec::new();

    for (idx, act) in doc.spec.activities.iter().enumerate() {
        node_map.insert(act.id.clone(), idx);

        let kind = NodeKind::from_str(&act.activity_type);
        let config = build_node_config(act);
        let mappings = compile_mappings(&act.input_mappings, &act.id)?;

        nodes.push(VilwNode {
            id: act.id.clone(),
            kind,
            output_variable: act.output_variable.clone(),
            durability: act.durability.clone(),
            config,
            mappings,
            compensation: act.compensation.as_ref()
                .map(|c| serde_json::to_value(c).unwrap_or_default()),
        });
    }

    // Add control nodes
    if let Some(ref controls) = doc.spec.controls {
        for ctrl in controls {
            let kind = match ctrl.control_type.as_deref() {
                Some("exclusive") => NodeKind::ExclusiveGateway,
                Some("inclusive") => NodeKind::InclusiveGateway,
                Some("parallel") => NodeKind::Parallel,
                Some("join") => NodeKind::Join,
                _ => NodeKind::Noop,
            };
            let idx = nodes.len();
            node_map.insert(ctrl.id.clone(), idx);
            nodes.push(VilwNode {
                id: ctrl.id.clone(),
                kind,
                output_variable: None,
                durability: None,
                config: serde_json::json!({}),
                mappings: Vec::new(),
                compensation: None,
            });
        }
    }

    // Always add implicit End if not present
    if !nodes.iter().any(|n| n.kind == NodeKind::End) {
        let idx = nodes.len();
        node_map.insert("end".into(), idx);
        nodes.push(VilwNode {
            id: "end".into(), kind: NodeKind::End, output_variable: None,
            durability: None, config: serde_json::json!({}),
            mappings: Vec::new(), compensation: None,
        });
    }

    // 3. Build edges
    let mut edges = Vec::new();
    for flow in &doc.spec.flows {
        let from_idx = node_map.get(&flow.from.node)
            .copied()
            .ok_or_else(|| CompileError {
                message: format!("flow {}: unknown from node '{}'", flow.id, flow.from.node),
                location: Some(format!("flow.{}", flow.id)),
            })?;
        let to_idx = node_map.get(&flow.to.node)
            .copied()
            .ok_or_else(|| CompileError {
                message: format!("flow {}: unknown to node '{}'", flow.id, flow.to.node),
                location: Some(format!("flow.{}", flow.id)),
            })?;

        // Validate guard condition if present
        if let Some(ref cond) = flow.condition {
            vil_expr::check_supported(cond).map_err(|e| CompileError {
                message: format!("guard condition: {}", e),
                location: Some(format!("flow.{}.condition", flow.id)),
            })?;
        }

        edges.push(VilwEdge {
            from_idx,
            to_idx,
            condition: flow.condition.clone(),
            priority: flow.priority.unwrap_or(0),
            detached: flow.detached.unwrap_or(false),
        });
    }

    // 4. Find entry node (first Trigger)
    let entry_node = nodes.iter().position(|n| n.kind == NodeKind::Trigger)
        .ok_or_else(|| CompileError {
            message: "no Trigger activity found".into(), location: None,
        })?;

    // 5. Extract metadata
    let id = doc.metadata.as_ref().and_then(|m| m.id.clone()).unwrap_or_else(|| "unnamed".into());
    let name = doc.metadata.as_ref().and_then(|m| m.name.clone()).unwrap_or_else(|| id.clone());

    let trigger_node = &nodes[entry_node];
    let trigger_config: Option<TriggerConfig> = serde_json::from_value(trigger_node.config.clone()).ok();
    let webhook_route = trigger_config.as_ref().and_then(|tc| tc.webhook_path());
    let webhook_method = trigger_config.as_ref()
        .and_then(|tc| tc.webhook_config.as_ref())
        .and_then(|wc| wc.method.clone())
        .unwrap_or_else(|| "POST".into())
        .to_uppercase();
    let trigger_type = trigger_config.as_ref()
        .and_then(|tc| tc.trigger_type.clone())
        .unwrap_or_else(|| "webhook".into());

    let durability_default = doc.spec.durability.as_ref()
        .and_then(|d| d.default_mode.clone())
        .unwrap_or_else(|| "eventual".into());

    let variables = doc.spec.variables.as_ref()
        .map(|vars| vars.iter().filter_map(|v| v.name.clone()).collect())
        .unwrap_or_default();

    Ok(VilwGraph {
        id, name, nodes, edges, variables,
        entry_node,
        durability_default,
        webhook_route,
        webhook_method,
        trigger_type,
    })
}

fn build_node_config(act: &VwfdActivity) -> serde_json::Value {
    if let Some(ref tc) = act.trigger_config {
        return serde_json::to_value(tc).unwrap_or_default();
    }
    if let Some(ref cc) = act.connector_config {
        return serde_json::to_value(cc).unwrap_or_default();
    }
    if let Some(ref rc) = act.rule_config {
        return serde_json::to_value(rc).unwrap_or_default();
    }
    if let Some(ref etc) = act.end_trigger_config {
        return serde_json::to_value(etc).unwrap_or_default();
    }
    if let Some(ref lc) = act.loop_config {
        return serde_json::to_value(lc).unwrap_or_default();
    }
    if let Some(ref wc) = act.wasm_config {
        return serde_json::to_value(wc).unwrap_or_default();
    }
    if let Some(ref sc) = act.sidecar_config {
        return serde_json::to_value(sc).unwrap_or_default();
    }
    if let Some(ref sw) = act.sub_workflow_config {
        return serde_json::to_value(sw).unwrap_or_default();
    }
    if let Some(ref ht) = act.human_task_config {
        return serde_json::to_value(ht).unwrap_or_default();
    }
    if let Some(ref nc) = act.code_config {
        return serde_json::to_value(nc).unwrap_or_default();
    }
    serde_json::json!({})
}

fn compile_mappings(
    mappings: &Option<Vec<InputMapping>>,
    activity_id: &str,
) -> Result<Vec<CompiledMapping>, CompileError> {
    let Some(maps) = mappings else { return Ok(Vec::new()); };
    let mut compiled = Vec::new();

    for m in maps {
        let target = m.target.as_deref().unwrap_or("").to_string();
        let source_obj = m.source.as_ref();
        let lang = source_obj.and_then(|s| s.language.as_deref()).unwrap_or("literal");
        let src = source_obj
            .and_then(|s| s.source.as_ref())
            .and_then(|v| match v {
                serde_yaml::Value::String(s) => Some(s.clone()),
                other => Some(format!("{:?}", other)),
            })
            .unwrap_or_default();

        // Validate and compile based on language
        match lang {
            "literal" | "spv1" => {
                compiled.push(CompiledMapping {
                    target, language: lang.into(), source: src,
                    compiled_sql: None, param_refs: None,
                });
            }
            "vil-expr" | "cel" => {
                // Validate VIL Expression expression is supported by vil_expr
                vil_expr::check_supported(&src).map_err(|e| CompileError {
                    message: e,
                    location: Some(format!("activity.{}.input_mappings.{}", activity_id, target)),
                })?;
                compiled.push(CompiledMapping {
                    target, language: "vil-expr".into(), source: src,
                    compiled_sql: None, param_refs: None,
                });
            }
            "vil_query" => {
                // Compile VilQuery DSL → SQL + param_refs at compile time
                let (sql, param_refs) = compile_vil_query(&src).map_err(|e| CompileError {
                    message: format!("vil_query compile: {}", e),
                    location: Some(format!("activity.{}.input_mappings.{}", activity_id, target)),
                })?;
                compiled.push(CompiledMapping {
                    target, language: "vil_query".into(), source: src,
                    compiled_sql: Some(sql),
                    param_refs: Some(param_refs),
                });
            }
            other => {
                return Err(CompileError {
                    message: format!(
                        "language '{}' not supported by vil compiler. \
                         Use vflow compile --cloud for vil-expr/vrule support.",
                        other
                    ),
                    location: Some(format!("activity.{}.input_mappings.{}", activity_id, target)),
                });
            }
        }
    }

    Ok(compiled)
}

/// Compile VilQuery inline DSL → SQL string + param_refs.
/// Reuses same DSL parser logic as vflow_compiler but outputs differently.
fn compile_vil_query(source: &str) -> Result<(String, Vec<String>), String> {
    // Parse the chain of method calls (same DSL as VFlow)
    let clean = source.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join("");

    let calls = parse_query_chain(&clean)?;

    let mut table = String::new();
    let mut columns = vec!["*".to_string()];
    let mut joins = Vec::new();
    let mut conditions = Vec::new();
    let mut param_refs = Vec::new();
    let mut order_clauses = Vec::new();
    let mut limit_val: Option<i64> = None;
    let mut offset_val: Option<i64> = None;
    let mut bind_counter = 0usize;

    for (method, args) in &calls {
        match method.as_str() {
            "select" => { table = unquote(&args[0]); }
            "columns" | "cols" => {
                columns = unquote(&args[0]).split(',').map(|s| s.trim().to_string()).collect();
            }
            "join" | "inner_join" => {
                let t = unquote(&args[0]);
                let on = if args.len() > 1 { unquote(&args[1]) } else { String::new() };
                joins.push(format!("JOIN {} ON {}", t, on));
            }
            "left_join" => {
                let t = unquote(&args[0]);
                let on = if args.len() > 1 { unquote(&args[1]) } else { String::new() };
                joins.push(format!("LEFT JOIN {} ON {}", t, on));
            }
            "where_eq" | "and_eq" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} = ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_gt" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} > ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_gte" | "where_ge" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} >= ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_lt" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} < ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_lte" | "where_le" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} <= ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_ne" | "where_neq" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} != ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_like" => {
                let col = unquote(&args[0]);
                bind_counter += 1;
                conditions.push(format!("{} LIKE ${}", col, bind_counter));
                param_refs.push(classify_value(&args[1]));
            }
            "where_null" => { conditions.push(format!("{} IS NULL", unquote(&args[0]))); }
            "where_not_null" => { conditions.push(format!("{} IS NOT NULL", unquote(&args[0]))); }
            "where_raw" => { conditions.push(unquote(&args[0])); }
            "order_by" => { order_clauses.push(unquote(&args[0])); }
            "order_by_asc" => { order_clauses.push(format!("{} ASC", unquote(&args[0]))); }
            "order_by_desc" => { order_clauses.push(format!("{} DESC", unquote(&args[0]))); }
            "limit" => { limit_val = args[0].trim().parse().ok(); }
            "offset" => { offset_val = args[0].trim().parse().ok(); }
            "group_by" => { /* TODO */ }
            _ => {}
        }
    }

    // Build SQL
    let mut sql = format!("SELECT {} FROM {}", columns.join(", "), table);
    for j in &joins { sql.push(' '); sql.push_str(j); }
    if !conditions.is_empty() { sql.push_str(" WHERE "); sql.push_str(&conditions.join(" AND ")); }
    if !order_clauses.is_empty() { sql.push_str(" ORDER BY "); sql.push_str(&order_clauses.join(", ")); }
    if let Some(l) = limit_val { sql.push_str(&format!(" LIMIT {}", l)); }
    if let Some(o) = offset_val { sql.push_str(&format!(" OFFSET {}", o)); }

    Ok((sql, param_refs))
}

fn classify_value(raw: &str) -> String {
    let t = raw.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        format!("_literal_str:{}", &t[1..t.len()-1])
    } else if t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok() {
        format!("_literal_num:{}", t)
    } else if t == "true" || t == "false" {
        format!("_literal_bool:{}", t)
    } else {
        t.to_string() // variable reference
    }
}

// ── VilQuery DSL Parser (same as vflow_compiler) ──

fn parse_query_chain(src: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut result = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut pos = 0;
    while pos < len && chars[pos].is_whitespace() { pos += 1; }
    loop {
        if pos >= len { break; }
        if chars[pos] == '.' { pos += 1; while pos < len && chars[pos].is_whitespace() { pos += 1; } }
        let ns = pos;
        while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') { pos += 1; }
        if pos == ns { break; }
        let method: String = chars[ns..pos].iter().collect();
        while pos < len && chars[pos].is_whitespace() { pos += 1; }
        if pos >= len || chars[pos] != '(' {
            return Err(format!("expected '(' after '{}'", method));
        }
        pos += 1;
        let args = parse_query_args(&chars, &mut pos)?;
        result.push((method, args));
        while pos < len && chars[pos].is_whitespace() { pos += 1; }
    }
    Ok(result)
}

fn parse_query_args(chars: &[char], pos: &mut usize) -> Result<Vec<String>, String> {
    let len = chars.len();
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 1;
    let mut in_string = false;
    let mut string_char = '"';
    while *pos < len && depth > 0 {
        let ch = chars[*pos];
        if in_string {
            current.push(ch);
            if ch == string_char && (*pos == 0 || chars[*pos - 1] != '\\') { in_string = false; }
        } else {
            match ch {
                '"' | '\'' => { in_string = true; string_char = ch; current.push(ch); }
                '(' => { depth += 1; current.push(ch); }
                ')' => { depth -= 1; if depth == 0 { let t = current.trim().to_string(); if !t.is_empty() { args.push(t); } } else { current.push(ch); } }
                ',' if depth == 1 => { args.push(current.trim().to_string()); current.clear(); }
                _ => { current.push(ch); }
            }
        }
        *pos += 1;
    }
    Ok(args)
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len()-1].to_string()
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_WORKFLOW: &str = r#"
version: "3.0"
metadata:
  id: test-simple
  name: "Simple Test"
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        response_mode: buffered
        end_activity: respond
        webhook_config:
          path: /test
      output_variable: trigger_payload

    - id: transform
      activity_type: Connector
      connector_config:
        connector_ref: vastar.http
        operation: post
      input_mappings:
        - target: url
          source:
            language: literal
            source: "http://api.example.com"
        - target: body
          source:
            language: vil-expr
            source: '{"name": trigger_payload.name, "active": true}'
      output_variable: result

    - id: respond
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response:
          language: vil-expr
          source: 'result'

    - id: end
      activity_type: End

  flows:
    - id: f1
      from: { node: trigger }
      to: { node: transform }
    - id: f2
      from: { node: transform }
      to: { node: respond }
    - id: f3
      from: { node: respond }
      to: { node: end }

  variables:
    - name: trigger_payload
      type: object
    - name: result
      type: object
"#;

    #[test]
    fn test_compile_simple() {
        let graph = compile(SIMPLE_WORKFLOW).unwrap();
        assert_eq!(graph.id, "test-simple");
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.entry_node, 0);
        assert_eq!(graph.webhook_route, Some("/test".into()));
        assert_eq!(graph.trigger_type, "webhook");
    }

    #[test]
    fn test_compile_mappings() {
        let graph = compile(SIMPLE_WORKFLOW).unwrap();
        let transform_node = &graph.nodes[1]; // transform
        assert_eq!(transform_node.mappings.len(), 2);
        assert_eq!(transform_node.mappings[0].language, "literal");
        assert_eq!(transform_node.mappings[1].language, "vil-expr");
    }

    #[test]
    fn test_compile_variables() {
        let graph = compile(SIMPLE_WORKFLOW).unwrap();
        assert!(graph.variables.contains(&"trigger_payload".to_string()));
        assert!(graph.variables.contains(&"result".to_string()));
    }

    #[test]
    fn test_reject_unsupported_vcel() {
        let yaml = r#"
version: "3.0"
metadata:
  id: test-reject
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config:
          path: /test
      output_variable: trigger_payload
    - id: transform
      activity_type: Connector
      connector_config:
        connector_ref: vastar.http
        operation: post
      input_mappings:
        - target: body
          source:
            language: vil-expr
            source: 'data.map(x, x * 2)'
    - id: end
      activity_type: End
  flows:
    - id: f1
      from: { node: trigger }
      to: { node: transform }
    - id: f2
      from: { node: transform }
      to: { node: end }
"#;
        let result = compile(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("VFlow"), "error should mention VFlow: {}", err.message);
    }

    const VIL_QUERY_WORKFLOW: &str = r#"
version: "3.0"
metadata:
  id: test-vilquery
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config:
          path: /query
      output_variable: trigger_payload
    - id: query
      activity_type: Connector
      connector_config:
        connector_ref: vastar.db.postgres
        operation: raw_query
      input_mappings:
        - target: query
          source:
            language: vil_query
            source: |
              select("users")
                .columns("id, name")
                .where_gt("score", trigger_payload.min_score)
                .order_by_desc("score")
                .limit(10)
      output_variable: query_result
    - id: end
      activity_type: End
  flows:
    - id: f1
      from: { node: trigger }
      to: { node: query }
    - id: f2
      from: { node: query }
      to: { node: end }
"#;

    #[test]
    fn test_compile_vil_query() {
        let graph = compile(VIL_QUERY_WORKFLOW).unwrap();
        let query_node = &graph.nodes[1];
        assert_eq!(query_node.mappings.len(), 1);
        assert_eq!(query_node.mappings[0].language, "vil_query");
        let sql = query_node.mappings[0].compiled_sql.as_ref().unwrap();
        assert!(sql.contains("SELECT id, name FROM users"));
        assert!(sql.contains("WHERE score > $1"));
        assert!(sql.contains("ORDER BY score DESC"));
        assert!(sql.contains("LIMIT 10"));
        let refs = query_node.mappings[0].param_refs.as_ref().unwrap();
        assert_eq!(refs[0], "trigger_payload.min_score");
    }

    #[test]
    fn test_graph_serialization() {
        let graph = compile(SIMPLE_WORKFLOW).unwrap();
        let bytes = graph.to_bytes();
        assert!(!bytes.is_empty());
        let restored = crate::graph::VilwGraph::from_bytes(&bytes).unwrap();
        assert_eq!(restored.id, graph.id);
        assert_eq!(restored.nodes.len(), graph.nodes.len());
    }
}
