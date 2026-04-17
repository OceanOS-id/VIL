//! # vil_expr — VIL Expression Evaluator (VIL Expression Compatible)
//!
//! Evaluates expressions against a variable map.
//! Expression syntax is VIL Expression compatible — expressions written here
//! will work without modification on VFlow's VIL Expression engine.
//!
//! ## Supported
//! - Path access: `trigger_payload.customer.name`
//! - Arithmetic: `+`, `-`, `*`, `/`, `%`
//! - Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
//! - Logical: `&&`, `||`, `!`
//! - Ternary: `cond ? a : b`
//! - Membership: `x in [1, 2, 3]`
//! - String concat: `"hello " + name`
//! - String methods: `.contains()`, `.startsWith()`, `.endsWith()`, `.size()`
//! - JSON templates: `{"name": trigger_payload.name, "active": true}`
//! - Functions: `size()`, `has()`, `int()`, `string()`, `double()`, `max()`, `min()`, `type()`
//! - List/Map construct and access
//!
//! ## Not Supported (requires VFlow VIL Expression)
//! - `.map()`, `.filter()`, `.all()`, `.exists()` (list macros)
//! - `matches()` (regex)
//! - `timestamp()`, `duration()` (temporal types)
//! - `bytes` type
//!
//! ## Example
//! ```
//! use vil_expr::evaluate;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut vars = HashMap::new();
//! vars.insert("name".into(), json!("Alice"));
//! vars.insert("score".into(), json!(85));
//!
//! assert_eq!(evaluate("name", &vars).unwrap(), json!("Alice"));
//! assert_eq!(evaluate("score > 80", &vars).unwrap(), json!(true));
//! assert_eq!(evaluate(r#""Hello " + name"#, &vars).unwrap(), json!("Hello Alice"));
//! ```

pub mod ast;
pub mod token;
pub mod parser;
pub mod eval;

use serde_json::Value;
use std::collections::HashMap;

pub type Vars = HashMap<String, Value>;

/// Evaluate VIL Expression expression → Value.
pub fn evaluate(expr: &str, vars: &Vars) -> Result<Value, String> {
    let parsed = parser::parse(expr)?;
    eval::eval(&parsed, vars)
}

/// Evaluate VIL Expression expression → bool.
pub fn evaluate_bool(expr: &str, vars: &Vars) -> Result<bool, String> {
    let val = evaluate(expr, vars)?;
    Ok(match &val {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        _ => true,
    })
}

/// Evaluate VIL Expression expression → String.
pub fn evaluate_to_string(expr: &str, vars: &Vars) -> Result<String, String> {
    let val = evaluate(expr, vars)?;
    Ok(match &val {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    })
}

/// Check if expression uses unsupported features (for compile-time validation).
pub fn check_supported(expr: &str) -> Result<(), String> {
    let parsed = parser::parse(expr)?;
    check_ast(&parsed)
}

fn check_ast(expr: &ast::Expr) -> Result<(), String> {
    match expr {
        ast::Expr::MethodCall(_, method, _) if
            matches!(method.as_str(), "map" | "filter" | "all" | "exists" | "exists_one" | "matches") => {
            Err(format!(".{}() requires VFlow VIL Expression engine. Use vflow compile --cloud", method))
        }
        ast::Expr::FnCall(name, _) if
            matches!(name.as_str(), "timestamp" | "duration" | "bytes") => {
            Err(format!("{}() requires VFlow VIL Expression engine. Use vflow compile --cloud", name))
        }
        // Recurse into children
        ast::Expr::Unary(_, e) => check_ast(e),
        ast::Expr::Binary(_, l, r) => { check_ast(l)?; check_ast(r) }
        ast::Expr::Ternary(c, t, e) => { check_ast(c)?; check_ast(t)?; check_ast(e) }
        ast::Expr::In(l, r) | ast::Expr::NotIn(l, r) => { check_ast(l)?; check_ast(r) }
        ast::Expr::IsNull(e) | ast::Expr::IsNotNull(e) => check_ast(e),
        ast::Expr::Field(e, _) => check_ast(e),
        ast::Expr::Index(e, i) => { check_ast(e)?; check_ast(i) }
        ast::Expr::FnCall(_, args) => { for a in args { check_ast(a)?; } Ok(()) }
        ast::Expr::MethodCall(e, _, args) => { check_ast(e)?; for a in args { check_ast(a)?; } Ok(()) }
        ast::Expr::List(items) => { for i in items { check_ast(i)?; } Ok(()) }
        ast::Expr::Map(entries) => { for (k, v) in entries { check_ast(k)?; check_ast(v)?; } Ok(()) }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vars() -> Vars {
        let mut v = HashMap::new();
        v.insert("trigger_payload".into(), json!({
            "name": "Alice",
            "age": 30,
            "score": 85,
            "items": [{"id": 1}, {"id": 2}, {"id": 3}],
            "address": {"city": "Jakarta", "zip": "12345"},
            "tags": ["vip", "active"],
            "active": true,
            "total": 150000
        }));
        v.insert("status".into(), json!("active"));
        v.insert("count".into(), json!(42));
        v
    }

    // ── Path access ──
    #[test] fn test_path() { assert_eq!(evaluate("trigger_payload.name", &vars()).unwrap(), json!("Alice")); }
    #[test] fn test_nested_path() { assert_eq!(evaluate("trigger_payload.address.city", &vars()).unwrap(), json!("Jakarta")); }
    #[test] fn test_simple_var() { assert_eq!(evaluate("status", &vars()).unwrap(), json!("active")); }

    // ── Arithmetic ──
    #[test] fn test_add() { assert_eq!(evaluate("count + 8", &vars()).unwrap(), json!(50)); }
    #[test] fn test_mul() { assert_eq!(evaluate("count * 2", &vars()).unwrap(), json!(84)); }
    #[test] fn test_mod() { assert_eq!(evaluate("count % 10", &vars()).unwrap(), json!(2)); }

    // ── String concat ──
    #[test] fn test_concat() { assert_eq!(evaluate(r#""Hello " + trigger_payload.name"#, &vars()).unwrap(), json!("Hello Alice")); }
    #[test] fn test_concat_path() { assert_eq!(evaluate(r#""http://host/" + trigger_payload.address.city"#, &vars()).unwrap(), json!("http://host/Jakarta")); }

    // ── Comparison ──
    #[test] fn test_gt() { assert_eq!(evaluate("trigger_payload.score > 80", &vars()).unwrap(), json!(true)); }
    #[test] fn test_eq() { assert_eq!(evaluate("status == 'active'", &vars()).unwrap(), json!(true)); }
    #[test] fn test_neq() { assert_eq!(evaluate("status != 'inactive'", &vars()).unwrap(), json!(true)); }
    #[test] fn test_gte() { assert_eq!(evaluate("trigger_payload.age >= 30", &vars()).unwrap(), json!(true)); }

    // ── Logical ──
    #[test] fn test_and() { assert_eq!(evaluate("trigger_payload.score > 80 && trigger_payload.active == true", &vars()).unwrap(), json!(true)); }
    #[test] fn test_or() { assert_eq!(evaluate("trigger_payload.score < 50 || trigger_payload.active == true", &vars()).unwrap(), json!(true)); }
    #[test] fn test_not() { assert_eq!(evaluate("!trigger_payload.active", &vars()).unwrap(), json!(false)); }

    // ── Ternary ──
    #[test] fn test_ternary_true() { assert_eq!(evaluate("trigger_payload.score > 70 ? 'pass' : 'fail'", &vars()).unwrap(), json!("pass")); }
    #[test] fn test_ternary_false() { assert_eq!(evaluate("trigger_payload.score > 90 ? 'excellent' : 'good'", &vars()).unwrap(), json!("good")); }

    // ── Membership ──
    #[test] fn test_in_list() { assert_eq!(evaluate("3 in [1, 2, 3]", &vars()).unwrap(), json!(true)); }
    #[test] fn test_not_in_list() { assert_eq!(evaluate("5 in [1, 2, 3]", &vars()).unwrap(), json!(false)); }

    // ── JSON template ──
    #[test] fn test_json_object() {
        let result = evaluate(r#"{"name": trigger_payload.name, "score": trigger_payload.score, "verified": true}"#, &vars()).unwrap();
        assert_eq!(result, json!({"name": "Alice", "score": 85, "verified": true}));
    }

    // ── Array construct ──
    #[test] fn test_array() {
        assert_eq!(evaluate("[1, 2, trigger_payload.age]", &vars()).unwrap(), json!([1, 2, 30]));
    }

    // ── Index access ──
    #[test] fn test_array_index() { assert_eq!(evaluate("trigger_payload.items[0].id", &vars()).unwrap(), json!(1)); }
    #[test] fn test_tags_index() { assert_eq!(evaluate("trigger_payload.tags[0]", &vars()).unwrap(), json!("vip")); }

    // ── Functions ──
    #[test] fn test_size_string() { assert_eq!(evaluate("size(trigger_payload.name)", &vars()).unwrap(), json!(5)); }
    #[test] fn test_size_list() { assert_eq!(evaluate("size(trigger_payload.items)", &vars()).unwrap(), json!(3)); }
    #[test] fn test_has_field() { assert_eq!(evaluate("has(trigger_payload.name)", &vars()).unwrap(), json!(true)); }
    #[test] fn test_has_missing() { assert_eq!(evaluate("has(trigger_payload.nonexistent)", &vars()).unwrap(), json!(false)); }
    #[test] fn test_int() { assert_eq!(evaluate("int(3.14)", &vars()).unwrap(), json!(3)); }
    #[test] fn test_string_fn() { assert_eq!(evaluate("string(42)", &vars()).unwrap(), json!("42")); }
    #[test] fn test_max() { assert_eq!(evaluate("max(1, 5, 3)", &vars()).unwrap(), json!(5)); }
    #[test] fn test_min() { assert_eq!(evaluate("min(10, 2, 8)", &vars()).unwrap(), json!(2)); }
    #[test] fn test_type() { assert_eq!(evaluate("type(trigger_payload.name)", &vars()).unwrap(), json!("string")); }

    // ── Method calls ──
    #[test] fn test_contains() { assert_eq!(evaluate("trigger_payload.name.contains('li')", &vars()).unwrap(), json!(true)); }
    #[test] fn test_starts_with() { assert_eq!(evaluate("trigger_payload.name.startsWith('Al')", &vars()).unwrap(), json!(true)); }
    #[test] fn test_ends_with() { assert_eq!(evaluate("trigger_payload.name.endsWith('ce')", &vars()).unwrap(), json!(true)); }
    #[test] fn test_string_size_method() { assert_eq!(evaluate("trigger_payload.name.size()", &vars()).unwrap(), json!(5)); }

    // ── List concat ──
    #[test] fn test_list_concat() { assert_eq!(evaluate("[1, 2] + [3, 4]", &vars()).unwrap(), json!([1, 2, 3, 4])); }

    // ── Unsupported detection ──
    #[test] fn test_unsupported_map() {
        assert!(check_supported("data.map(x, x * 2)").is_err());
    }
    #[test] fn test_unsupported_filter() {
        assert!(check_supported("items.filter(x, x > 0)").is_err());
    }
    #[test] fn test_supported_basic() {
        assert!(check_supported("trigger_payload.name == 'Alice' && score > 80").is_ok());
    }

    // ── evaluate_bool ──
    #[test] fn test_eval_bool() { assert!(evaluate_bool("trigger_payload.score > 80", &vars()).unwrap()); }
    #[test] fn test_eval_bool_false() { assert!(!evaluate_bool("trigger_payload.score > 90", &vars()).unwrap()); }

    // ── evaluate_to_string ──
    #[test] fn test_eval_string() { assert_eq!(evaluate_to_string("trigger_payload.name", &vars()).unwrap(), "Alice"); }

    // ── Complex real-world ──
    #[test] fn test_real_world_guard() {
        assert!(evaluate_bool("trigger_payload.total >= 100000 && trigger_payload.active == true", &vars()).unwrap());
    }
    #[test] fn test_real_world_json_response() {
        let result = evaluate(r#"{"customer": trigger_payload.name, "amount": trigger_payload.total, "status": status}"#, &vars()).unwrap();
        assert_eq!(result["customer"], "Alice");
        assert_eq!(result["amount"], 150000);
        assert_eq!(result["status"], "active");
    }

    // ── vdicl expression extensions ──

    fn vdicl_vars() -> Vars {
        let mut v = HashMap::new();
        v.insert("nasabah".into(), json!({
            "nik": "1234567890123456",
            "nama": "Budi Santoso",
            "status_kawin": "K",
            "nik_pasangan": null,
        }));
        v.insert("fasilitas".into(), json!({
            "plafon": 5000000000i64,
            "baki_debet": 6000000000i64,
            "kolektibilitas": "4",
            "suku_bunga": 12.5,
        }));
        v.insert("tenant_id".into(), json!(null));
        v.insert("branch_id".into(), json!(""));
        v
    }

    // IS NULL / IS NOT NULL
    #[test] fn test_is_null() { assert!(evaluate_bool("tenant_id IS NULL", &vdicl_vars()).unwrap()); }
    #[test] fn test_is_not_null() { assert!(evaluate_bool("nasabah.nik IS NOT NULL", &vdicl_vars()).unwrap()); }
    #[test] fn test_is_null_nested() { assert!(evaluate_bool("nasabah.nik_pasangan IS NULL", &vdicl_vars()).unwrap()); }

    // ISBLANK / LENGTH
    #[test] fn test_isblank_null() { assert!(evaluate_bool("ISBLANK(tenant_id)", &vdicl_vars()).unwrap()); }
    #[test] fn test_isblank_empty() { assert!(evaluate_bool("ISBLANK(branch_id)", &vdicl_vars()).unwrap()); }
    #[test] fn test_isblank_not() { assert!(!evaluate_bool("ISBLANK(nasabah.nik)", &vdicl_vars()).unwrap()); }
    #[test] fn test_length() { assert_eq!(evaluate("LENGTH(nasabah.nik)", &vdicl_vars()).unwrap(), json!(16)); }

    // AND / OR / NOT keywords
    #[test] fn test_and_keyword() { assert!(evaluate_bool("nasabah.nik IS NOT NULL AND nasabah.nama IS NOT NULL", &vdicl_vars()).unwrap()); }
    #[test] fn test_or_keyword() { assert!(evaluate_bool("tenant_id IS NULL OR nasabah.nik IS NOT NULL", &vdicl_vars()).unwrap()); }
    #[test] fn test_not_keyword() { assert!(evaluate_bool("NOT ISBLANK(nasabah.nik)", &vdicl_vars()).unwrap()); }

    // IN {set} / NOT IN
    #[test] fn test_in_set() { assert!(evaluate_bool("fasilitas.kolektibilitas IN {'4', '5'}", &vdicl_vars()).unwrap()); }
    #[test] fn test_not_in_set() { assert!(evaluate_bool("fasilitas.kolektibilitas NOT IN {'1', '2', '3'}", &vdicl_vars()).unwrap()); }

    // Decimal suffix m
    #[test] fn test_decimal_m() { assert!(evaluate_bool("fasilitas.plafon > 1000000000m", &vdicl_vars()).unwrap()); }

    // Cross-field logic (SLIK-style)
    #[test] fn test_slik_cross_field() {
        assert!(evaluate_bool(
            "nasabah.status_kawin == 'K' AND (nasabah.nik_pasangan IS NULL OR ISBLANK(nasabah.nik_pasangan))",
            &vdicl_vars()
        ).unwrap());
    }
    #[test] fn test_slik_baki_debet_gt_plafon() {
        assert!(evaluate_bool("fasilitas.baki_debet > fasilitas.plafon", &vdicl_vars()).unwrap());
    }
    #[test] fn test_slik_kolektibilitas_in_bad() {
        assert!(evaluate_bool("fasilitas.kolektibilitas IN {'4', '5'} AND fasilitas.baki_debet > 0m", &vdicl_vars()).unwrap());
    }
}
