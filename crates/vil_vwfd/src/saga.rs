//! Saga compensation — reverse-walk completed steps on failure.
//!
//! Each VWFD activity can define `compensation` config.
//! On workflow failure: walk completed nodes in reverse,
//! execute compensation connector calls.

use crate::graph::VilwGraph;
use crate::executor::ExecConfig;
use serde_json::Value;
use std::collections::HashMap;

/// Compensation action collected during execution.
#[derive(Debug, Clone)]
pub struct CompensationAction {
    pub node_id: String,
    pub connector_ref: String,
    pub operation: String,
    pub input_mappings: Value,
}

/// Collect compensation actions from completed nodes.
pub fn collect_compensations(
    graph: &VilwGraph,
    completed_nodes: &[String],
) -> Vec<CompensationAction> {
    let mut actions = Vec::new();
    for node_id in completed_nodes {
        if let Some(node) = graph.nodes.iter().find(|n| n.id == *node_id) {
            if let Some(ref comp) = node.compensation {
                if let (Some(cref), Some(op)) = (
                    comp.get("connector_ref").and_then(|v| v.as_str()),
                    comp.get("operation").and_then(|v| v.as_str()),
                ) {
                    actions.push(CompensationAction {
                        node_id: node_id.clone(),
                        connector_ref: cref.into(),
                        operation: op.into(),
                        input_mappings: comp.get("input_mappings").cloned().unwrap_or(Value::Array(Vec::new())),
                    });
                }
            }
        }
    }
    actions
}

/// Execute saga compensation in reverse order (async).
pub async fn run_compensation(
    actions: &[CompensationAction],
    _vars: &HashMap<String, Value>,
    config: &ExecConfig,
) -> Vec<CompensationResult> {
    let mut results = Vec::new();

    for action in actions.iter().rev() {
        let result = if let Some(ref connector_fn) = config.connector_fn {
            let input = serde_json::json!({
                "_compensation": true,
                "_original_node": action.node_id,
            });
            match connector_fn(&action.connector_ref, &action.operation, &input).await {
                Ok(_) => CompensationResult {
                    node_id: action.node_id.clone(),
                    success: true,
                    error: None,
                },
                Err(e) => CompensationResult {
                    node_id: action.node_id.clone(),
                    success: false,
                    error: Some(e),
                },
            }
        } else {
            // No connector function — log compensation intent
            CompensationResult {
                node_id: action.node_id.clone(),
                success: true,
                error: None,
            }
        };

        results.push(result);
    }

    results
}

#[derive(Debug, Clone)]
pub struct CompensationResult {
    pub node_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler;
    use serde_json::json;

    const SAGA_WF: &str = r#"
version: "3.0"
metadata:
  id: test-saga
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /saga }
      output_variable: trigger_payload
    - id: step_a
      activity_type: Connector
      connector_config: { connector_ref: vastar.http, operation: post }
      output_variable: step_a_result
      compensation:
        connector_ref: vastar.http
        operation: post
        input_mappings: []
    - id: step_b
      activity_type: Connector
      connector_config: { connector_ref: vastar.db.postgres, operation: insert }
      output_variable: step_b_result
      compensation:
        connector_ref: vastar.db.postgres
        operation: delete
        input_mappings: []
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: step_a } }
    - { id: f2, from: { node: step_a }, to: { node: step_b } }
    - { id: f3, from: { node: step_b }, to: { node: end } }
"#;

    #[test]
    fn test_collect_compensations() {
        let graph = compiler::compile(SAGA_WF).unwrap();
        let completed = vec!["step_a".into(), "step_b".into()];
        let actions = collect_compensations(&graph, &completed);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].node_id, "step_a");
        assert_eq!(actions[0].connector_ref, "vastar.http");
        assert_eq!(actions[1].node_id, "step_b");
        assert_eq!(actions[1].connector_ref, "vastar.db.postgres");
        assert_eq!(actions[1].operation, "delete");
    }

    #[tokio::test]
    async fn test_run_compensation_reverse() {
        let graph = compiler::compile(SAGA_WF).unwrap();
        let completed = vec!["step_a".into(), "step_b".into()];
        let actions = collect_compensations(&graph, &completed);

        let config = ExecConfig {
            connector_fn: Some(std::sync::Arc::new(move |cref, op, _input| {
                let (cref, op) = (cref.to_string(), op.to_string());
                Box::pin(async move {
                    Ok(json!({"compensated": true, "ref": cref, "op": op}))
                })
            })),
            ..Default::default()
        };

        let results = run_compensation(&actions, &HashMap::new(), &config).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].node_id, "step_b");
        assert_eq!(results[1].node_id, "step_a");
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_no_compensation() {
        let graph = compiler::compile(SAGA_WF).unwrap();
        let completed = vec!["trigger".into()]; // trigger has no compensation
        let actions = collect_compensations(&graph, &completed);
        assert!(actions.is_empty());
    }
}
