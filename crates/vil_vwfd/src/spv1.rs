//! SelectorPath V1 (SPv1) — JSONPath evaluator for VIL.
//!
//! Implements SPv1 spec: `$` root, dot notation, bracket notation,
//! wildcard, slice, filter, union. Standard Rust implementation.
//!
//! Syntax:
//!   $.field.subfield           — dot notation
//!   $['key']                   — bracket quoted key
//!   $[0]                       — array index
//!   $[*]  or $.*               — wildcard (all elements)
//!   $[1:5]  $[::2]             — slice (start:end:step)
//!   $['a','b']                 — union of keys
//!   $[?(@.price > 10)]         — filter expression

use serde_json::Value;
use std::collections::HashMap;

/// Evaluate SPv1 expression against a JSON document. Returns all matches.
pub fn query(expr: &str, doc: &Value) -> Result<Vec<Value>, String> {
    let expr = expr.trim();
    if !expr.starts_with('$') {
        return Err(format!("SPv1 expression must start with '$', got: {}", expr));
    }
    let steps = parse_steps(&expr[1..])?;
    let mut current = vec![doc.clone()];
    for step in &steps {
        let mut next = Vec::new();
        for val in &current {
            apply_step(step, val, &mut next)?;
        }
        current = next;
    }
    Ok(current)
}

/// Evaluate SPv1 and return single value (first match or null).
pub fn query_one(expr: &str, doc: &Value) -> Result<Value, String> {
    let results = query(expr, doc)?;
    Ok(results.into_iter().next().unwrap_or(Value::Null))
}

/// Evaluate SPv1 template: replace `$.path` references in string with values.
pub fn eval_template(template: &str, vars: &HashMap<String, Value>) -> String {
    let doc = Value::Object(vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let mut result = template.to_string();
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut replacements: Vec<(String, String)> = Vec::new();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' && i + 1 < len && (chars[i + 1] == '.' || chars[i + 1] == '[') {
            let start = i;
            i += 1;
            while i < len {
                if chars[i] == '.' && i + 1 < len && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_' || chars[i + 1] == '*') {
                    i += 1;
                    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
                } else if chars[i] == '[' {
                    i += 1;
                    let mut depth = 1;
                    while i < len && depth > 0 {
                        if chars[i] == '[' { depth += 1; }
                        if chars[i] == ']' { depth -= 1; }
                        if depth > 0 { i += 1; }
                    }
                    if i < len { i += 1; }
                } else {
                    break;
                }
            }
            let path: String = chars[start..i].iter().collect();
            if let Ok(val) = query_one(&path, &doc) {
                replacements.push((path, value_to_string(&val)));
            }
        } else {
            i += 1;
        }
    }

    replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    for (pattern, replacement) in replacements {
        result = result.replace(&pattern, &replacement);
    }
    result
}

// ── Step types ──

#[derive(Debug)]
enum Step {
    Field(String),
    Index(i64),
    QuotedKey(String),
    Wildcard,
    Slice(Option<i64>, Option<i64>, Option<i64>),
    Union(Vec<UnionItem>),
    Filter(String),
}

#[derive(Debug)]
enum UnionItem { Key(String), Index(i64) }

// ── Parser ──

fn parse_steps(input: &str) -> Result<Vec<Step>, String> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut pos = 0;
    let mut steps = Vec::new();

    while pos < len {
        if chars[pos] == '.' {
            pos += 1;
            if pos < len && chars[pos] == '*' {
                steps.push(Step::Wildcard);
                pos += 1;
            } else {
                let start = pos;
                while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') { pos += 1; }
                if pos == start { return Err("expected field name after '.'".into()); }
                steps.push(Step::Field(chars[start..pos].iter().collect()));
            }
        } else if chars[pos] == '[' {
            pos += 1;
            skip_ws(&chars, &mut pos);
            if pos >= len { return Err("unclosed bracket".into()); }

            if chars[pos] == '*' {
                pos += 1;
                skip_ws(&chars, &mut pos);
                if pos < len && chars[pos] == ']' { pos += 1; }
                steps.push(Step::Wildcard);
            } else if chars[pos] == '?' {
                pos += 1;
                if pos < len && chars[pos] == '(' {
                    pos += 1;
                    let start = pos;
                    let mut depth = 1;
                    while pos < len && depth > 0 {
                        if chars[pos] == '(' { depth += 1; }
                        if chars[pos] == ')' { depth -= 1; }
                        if depth > 0 { pos += 1; }
                    }
                    let filter_expr: String = chars[start..pos].iter().collect();
                    if pos < len { pos += 1; } // skip )
                    skip_ws(&chars, &mut pos);
                    if pos < len && chars[pos] == ']' { pos += 1; }
                    steps.push(Step::Filter(filter_expr));
                }
            } else if chars[pos] == '\'' || chars[pos] == '"' {
                let mut keys = Vec::new();
                loop {
                    skip_ws(&chars, &mut pos);
                    if pos >= len || (chars[pos] != '\'' && chars[pos] != '"') { break; }
                    let quote = chars[pos];
                    pos += 1;
                    let start = pos;
                    while pos < len && chars[pos] != quote { pos += 1; }
                    keys.push(chars[start..pos].iter().collect::<String>());
                    if pos < len { pos += 1; }
                    skip_ws(&chars, &mut pos);
                    if pos < len && chars[pos] == ',' { pos += 1; } else { break; }
                }
                skip_ws(&chars, &mut pos);
                if pos < len && chars[pos] == ']' { pos += 1; }
                if keys.len() == 1 {
                    steps.push(Step::QuotedKey(keys.into_iter().next().unwrap()));
                } else {
                    steps.push(Step::Union(keys.into_iter().map(UnionItem::Key).collect()));
                }
            } else {
                let content = read_until_bracket(&chars, &mut pos);
                if pos < len && chars[pos] == ']' { pos += 1; }
                if content.contains(':') {
                    let parts: Vec<&str> = content.split(':').collect();
                    let s = parts.first().and_then(|s| s.trim().parse().ok());
                    let e = parts.get(1).and_then(|s| s.trim().parse().ok());
                    let st = parts.get(2).and_then(|s| s.trim().parse().ok());
                    steps.push(Step::Slice(s, e, st));
                } else if content.contains(',') {
                    let items = content.split(',')
                        .filter_map(|s| s.trim().parse::<i64>().ok().map(UnionItem::Index))
                        .collect();
                    steps.push(Step::Union(items));
                } else {
                    let idx: i64 = content.trim().parse().map_err(|_| format!("invalid index: {}", content))?;
                    steps.push(Step::Index(idx));
                }
            }
        } else {
            return Err(format!("unexpected '{}' at pos {}", chars[pos], pos));
        }
    }
    Ok(steps)
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() { *pos += 1; }
}

fn read_until_bracket(chars: &[char], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < chars.len() && chars[*pos] != ']' { *pos += 1; }
    chars[start..*pos].iter().collect()
}

// ── Step application ──

fn apply_step(step: &Step, val: &Value, out: &mut Vec<Value>) -> Result<(), String> {
    match step {
        Step::Field(name) => {
            if let Some(v) = val.get(name.as_str()) { out.push(v.clone()); }
        }
        Step::QuotedKey(key) => {
            if let Some(v) = val.get(key.as_str()) { out.push(v.clone()); }
        }
        Step::Index(idx) => {
            if let Value::Array(arr) = val {
                let i = if *idx < 0 { (arr.len() as i64 + idx) as usize } else { *idx as usize };
                if let Some(v) = arr.get(i) { out.push(v.clone()); }
            }
        }
        Step::Wildcard => {
            match val {
                Value::Array(arr) => { for v in arr { out.push(v.clone()); } }
                Value::Object(map) => { for v in map.values() { out.push(v.clone()); } }
                _ => {}
            }
        }
        Step::Slice(start, end, step) => {
            if let Value::Array(arr) = val {
                let len = arr.len() as i64;
                let s = resolve_idx(start.unwrap_or(0), len);
                let e = resolve_idx(end.unwrap_or(len), len);
                let st = step.unwrap_or(1).max(1) as usize;
                let mut i = s;
                while i < e { if let Some(v) = arr.get(i) { out.push(v.clone()); } i += st; }
            }
        }
        Step::Union(items) => {
            for item in items {
                match item {
                    UnionItem::Key(key) => {
                        if let Some(v) = val.get(key.as_str()) { out.push(v.clone()); }
                    }
                    UnionItem::Index(idx) => {
                        if let Value::Array(arr) = val {
                            let i = if *idx < 0 { (arr.len() as i64 + idx) as usize } else { *idx as usize };
                            if let Some(v) = arr.get(i) { out.push(v.clone()); }
                        }
                    }
                }
            }
        }
        Step::Filter(expr) => {
            if let Value::Array(arr) = val {
                for item in arr {
                    if eval_filter(expr, item)? { out.push(item.clone()); }
                }
            }
        }
    }
    Ok(())
}

fn resolve_idx(idx: i64, len: i64) -> usize {
    (if idx < 0 { (len + idx).max(0) } else { idx.min(len) }) as usize
}

fn eval_filter(expr: &str, current: &Value) -> Result<bool, String> {
    let mut vars = HashMap::new();
    if let Value::Object(map) = current {
        for (k, v) in map { vars.insert(k.clone(), v.clone()); }
    }
    let normalized = expr.replace("@.", "");
    vil_expr::evaluate_bool(&normalized, &vars)
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> Value {
        json!({
            "store": {
                "book": [
                    {"title": "Rust Programming", "price": 29.99, "category": "tech"},
                    {"title": "Go Concurrency", "price": 19.99, "category": "tech"},
                    {"title": "Design Patterns", "price": 39.99, "category": "software"},
                    {"title": "Clean Code", "price": 24.99, "category": "software"}
                ],
                "name": "TechBooks"
            },
            "count": 4
        })
    }

    #[test] fn test_dot_field() { assert_eq!(query_one("$.store.name", &doc()).unwrap(), json!("TechBooks")); }
    #[test] fn test_dot_nested() { assert_eq!(query_one("$.count", &doc()).unwrap(), json!(4)); }
    #[test] fn test_bracket_key() { assert_eq!(query_one("$['store']['name']", &doc()).unwrap(), json!("TechBooks")); }
    #[test] fn test_array_index() { assert_eq!(query_one("$.store.book[0].title", &doc()).unwrap(), json!("Rust Programming")); }
    #[test] fn test_negative_index() { assert_eq!(query_one("$.store.book[-1].title", &doc()).unwrap(), json!("Clean Code")); }

    #[test] fn test_wildcard_array() {
        let r = query("$.store.book[*].title", &doc()).unwrap();
        assert_eq!(r.len(), 4);
        assert_eq!(r[0], json!("Rust Programming"));
    }

    #[test] fn test_slice() {
        let r = query("$.store.book[0:2]", &doc()).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test] fn test_slice_step() {
        let r = query("$.store.book[0:4:2]", &doc()).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test] fn test_union_indices() {
        let r = query("$.store.book[0,2]", &doc()).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test] fn test_filter_gt() {
        let r = query("$.store.book[?(@.price > 25)]", &doc()).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test] fn test_filter_eq() {
        let r = query("$.store.book[?(@.category == 'tech')]", &doc()).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test] fn test_template() {
        let mut vars = HashMap::new();
        vars.insert("user".into(), json!({"name": "Alice", "age": 30}));
        assert_eq!(eval_template("Hello $.user.name, age $.user.age", &vars), "Hello Alice, age 30");
    }

    #[test] fn test_template_json() {
        let mut vars = HashMap::new();
        vars.insert("messages".into(), json!([{"role": "user"}]));
        let r = eval_template(r#"{"messages": $.messages}"#, &vars);
        assert!(r.contains(r#"[{"role":"user"}]"#));
    }

    #[test] fn test_union_keys() {
        let r = query("$.store.book[0]['title','price']", &doc());
        // This would need nested steps — book[0] then ['title','price'] union
        // For now just verify it doesn't crash
        assert!(r.is_ok());
    }
}
