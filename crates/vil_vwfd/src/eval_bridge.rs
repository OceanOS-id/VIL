//! Eval bridge — map VWFD language tags to appropriate evaluator.

use crate::graph::CompiledMapping;
use serde_json::Value;
use std::collections::HashMap;

/// Evaluate a compiled mapping against variable store.
pub fn eval_mapping(
    mapping: &CompiledMapping,
    vars: &HashMap<String, Value>,
) -> Result<Value, String> {
    match mapping.language.as_str() {
        "literal" => {
            // Try parse as JSON first, fallback to string
            Ok(serde_json::from_str::<Value>(&mapping.source)
                .unwrap_or_else(|_| Value::String(mapping.source.clone())))
        }

        "spv1" => {
            let result = crate::spv1::eval_template(&mapping.source, vars);
            // Try parse result as JSON
            Ok(serde_json::from_str(&result).unwrap_or(Value::String(result)))
        }

        "vil-expr" | "cel" => {
            vil_expr::evaluate(&mapping.source, vars)
        }

        "vil_query" => {
            // Pre-compiled SQL — resolve param_refs at runtime
            if let Some(ref sql) = mapping.compiled_sql {
                let mut params = Vec::new();
                if let Some(ref refs) = mapping.param_refs {
                    for r in refs {
                        let val = resolve_param_ref(r, vars);
                        params.push(val);
                    }
                }
                Ok(serde_json::json!({
                    "operation": "raw_query",
                    "sql": sql,
                    "params": params,
                    "_vil_query": true,
                }))
            } else {
                Err("vil_query mapping has no compiled_sql".into())
            }
        }

        other => Err(format!(
            "language '{}' not supported. Use vflow compile --cloud for full VIL Expression/Rule.",
            other
        )),
    }
}

/// Resolve a param_ref — literal or variable path.
fn resolve_param_ref(ref_str: &str, vars: &HashMap<String, Value>) -> Value {
    if let Some(val) = ref_str.strip_prefix("_literal_str:") {
        return Value::String(val.to_string());
    }
    if let Some(val) = ref_str.strip_prefix("_literal_num:") {
        if let Ok(n) = val.parse::<i64>() { return Value::Number(n.into()); }
        if let Ok(n) = val.parse::<f64>() {
            return serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null);
        }
        return Value::String(val.to_string());
    }
    if let Some(val) = ref_str.strip_prefix("_literal_bool:") {
        return Value::Bool(val == "true");
    }

    // Variable path: trigger_payload.min_amount
    let parts: Vec<&str> = ref_str.splitn(2, '.').collect();
    if let Some(root) = vars.get(parts[0]) {
        if parts.len() == 1 { return root.clone(); }
        let mut current = root.clone();
        for key in parts[1].split('.') {
            current = match current {
                Value::Object(ref obj) => obj.get(key).cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };
        }
        current
    } else {
        Value::Null
    }
}

/// Evaluate all mappings for an activity → produce key-value input.
pub fn eval_all_mappings(
    mappings: &[CompiledMapping],
    vars: &HashMap<String, Value>,
) -> Result<HashMap<String, Value>, String> {
    let mut result = HashMap::new();
    for m in mappings {
        let val = eval_mapping(m, vars)?;
        // For vil_query: flatten into top-level
        if m.language == "vil_query" {
            if let Some(obj) = val.as_object() {
                for (k, v) in obj {
                    result.insert(k.clone(), v.clone());
                }
                continue;
            }
        }
        result.insert(m.target.clone(), val);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_vars() -> HashMap<String, Value> {
        let mut v = HashMap::new();
        v.insert("trigger_payload".into(), json!({"name": "Alice", "amount": 100}));
        v.insert("status".into(), json!("active"));
        v
    }

    #[test]
    fn test_literal() {
        let m = CompiledMapping {
            target: "url".into(), language: "literal".into(),
            source: "http://example.com".into(),
            compiled_sql: None, param_refs: None,
        };
        let result = eval_mapping(&m, &test_vars()).unwrap();
        assert_eq!(result, json!("http://example.com"));
    }

    #[test]
    fn test_vcel() {
        let m = CompiledMapping {
            target: "body".into(), language: "vil-expr".into(),
            source: r#"{"name": trigger_payload.name, "total": trigger_payload.amount}"#.into(),
            compiled_sql: None, param_refs: None,
        };
        let result = eval_mapping(&m, &test_vars()).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["total"], 100);
    }

    #[test]
    fn test_spv1() {
        let m = CompiledMapping {
            target: "greeting".into(), language: "spv1".into(),
            source: "Hello $.trigger_payload.name".into(),
            compiled_sql: None, param_refs: None,
        };
        let result = eval_mapping(&m, &test_vars()).unwrap();
        assert_eq!(result, json!("Hello Alice"));
    }

    #[test]
    fn test_vil_query() {
        let m = CompiledMapping {
            target: "query".into(), language: "vil_query".into(),
            source: "original DSL".into(),
            compiled_sql: Some("SELECT * FROM users WHERE amount > $1".into()),
            param_refs: Some(vec!["trigger_payload.amount".into()]),
        };
        let result = eval_mapping(&m, &test_vars()).unwrap();
        assert_eq!(result["sql"], "SELECT * FROM users WHERE amount > $1");
        assert_eq!(result["params"][0], 100);
    }
}
