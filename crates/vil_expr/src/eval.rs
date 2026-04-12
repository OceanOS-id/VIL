/// Evaluator — walk AST against variable map, produce serde_json::Value.

use crate::ast::*;
use serde_json::Value;
use std::collections::HashMap;

pub type Vars = HashMap<String, Value>;

pub fn eval(expr: &Expr, vars: &Vars) -> Result<Value, String> {
    match expr {
        // ── Literals ──
        Expr::Int(n) => Ok(Value::Number((*n).into())),
        Expr::Float(n) => Ok(serde_json::Number::from_f64(*n)
            .map(Value::Number).unwrap_or(Value::Null)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Null => Ok(Value::Null),

        // ── Collections ──
        Expr::List(items) => {
            let vals: Result<Vec<Value>, _> = items.iter().map(|e| eval(e, vars)).collect();
            Ok(Value::Array(vals?))
        }
        Expr::Map(entries) => {
            let mut map = serde_json::Map::new();
            for (k, v) in entries {
                let key = val_to_string(&eval(k, vars)?);
                let val = eval(v, vars)?;
                map.insert(key, val);
            }
            Ok(Value::Object(map))
        }

        // ── Ident (variable lookup) ──
        Expr::Ident(name) => {
            Ok(vars.get(name).cloned().unwrap_or(Value::Null))
        }

        // ── Field access: expr.field ──
        Expr::Field(obj, field) => {
            let val = eval(obj, vars)?;
            Ok(field_access(&val, field))
        }

        // ── Index: expr[index] ──
        Expr::Index(obj, idx) => {
            let val = eval(obj, vars)?;
            let i = eval(idx, vars)?;
            match (&val, &i) {
                (Value::Array(arr), Value::Number(n)) => {
                    let idx = n.as_u64().unwrap_or(0) as usize;
                    Ok(arr.get(idx).cloned().unwrap_or(Value::Null))
                }
                (Value::Object(map), Value::String(key)) => {
                    Ok(map.get(key).cloned().unwrap_or(Value::Null))
                }
                _ => Ok(Value::Null),
            }
        }

        // ── Unary ──
        Expr::Unary(op, e) => {
            let v = eval(e, vars)?;
            match op {
                UnaryOp::Not => Ok(Value::Bool(!val_to_bool(&v))),
                UnaryOp::Neg => match &v {
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() { Ok(Value::Number((-i).into())) }
                        else if let Some(f) = n.as_f64() {
                            Ok(serde_json::Number::from_f64(-f).map(Value::Number).unwrap_or(Value::Null))
                        }
                        else { Ok(Value::Null) }
                    }
                    _ => Err(format!("cannot negate {:?}", v)),
                }
            }
        }

        // ── Binary ──
        Expr::Binary(op, left, right) => {
            let l = eval(left, vars)?;
            // Short-circuit for && and ||
            match op {
                BinaryOp::And => {
                    if !val_to_bool(&l) { return Ok(Value::Bool(false)); }
                    let r = eval(right, vars)?;
                    return Ok(Value::Bool(val_to_bool(&r)));
                }
                BinaryOp::Or => {
                    if val_to_bool(&l) { return Ok(Value::Bool(true)); }
                    let r = eval(right, vars)?;
                    return Ok(Value::Bool(val_to_bool(&r)));
                }
                _ => {}
            }
            let r = eval(right, vars)?;
            eval_binary(*op, &l, &r)
        }

        // ── Ternary ──
        Expr::Ternary(cond, then, else_) => {
            if val_to_bool(&eval(cond, vars)?) {
                eval(then, vars)
            } else {
                eval(else_, vars)
            }
        }

        // ── In ──
        Expr::In(item, collection) => {
            let item_val = eval(item, vars)?;
            let coll_val = eval(collection, vars)?;
            match &coll_val {
                Value::Array(arr) => Ok(Value::Bool(arr.contains(&item_val))),
                Value::Object(map) => {
                    let key = val_to_string(&item_val);
                    Ok(Value::Bool(map.contains_key(&key)))
                }
                _ => Ok(Value::Bool(false)),
            }
        }

        // ── Function call ──
        Expr::FnCall(name, args) => {
            eval_function(name, args, vars)
        }

        // ── Method call ──
        Expr::MethodCall(obj, method, args) => {
            let obj_val = eval(obj, vars)?;
            let arg_vals: Result<Vec<Value>, _> = args.iter().map(|a| eval(a, vars)).collect();
            eval_method(&obj_val, method, &arg_vals?)
        }
    }
}

// ── Binary operator evaluation ──

fn eval_binary(op: BinaryOp, l: &Value, r: &Value) -> Result<Value, String> {
    match op {
        // String concat or numeric add
        BinaryOp::Add => {
            if l.is_string() || r.is_string() {
                Ok(Value::String(val_to_string(l) + &val_to_string(r)))
            } else if let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) {
                // If both are integers, keep as integer
                if l.is_i64() && r.is_i64() {
                    Ok(Value::Number((l.as_i64().unwrap() + r.as_i64().unwrap()).into()))
                } else {
                    Ok(serde_json::Number::from_f64(a + b).map(Value::Number).unwrap_or(Value::Null))
                }
            } else if let (Some(a), Some(b)) = (l.as_array(), r.as_array()) {
                // List concat
                let mut combined = a.clone();
                combined.extend(b.iter().cloned());
                Ok(Value::Array(combined))
            } else {
                Ok(Value::String(val_to_string(l) + &val_to_string(r)))
            }
        }
        BinaryOp::Sub => num_op(l, r, |a, b| a - b, |a, b| a - b),
        BinaryOp::Mul => num_op(l, r, |a, b| a * b, |a, b| a * b),
        BinaryOp::Div => {
            if r.as_f64() == Some(0.0) { return Err("division by zero".into()); }
            num_op(l, r, |a, b| a / b, |a, b| a / b)
        }
        BinaryOp::Mod => num_op(l, r, |a, b| a % b, |a, b| a % b),

        // Comparison
        BinaryOp::Eq => Ok(Value::Bool(val_eq(l, r))),
        BinaryOp::Neq => Ok(Value::Bool(!val_eq(l, r))),
        BinaryOp::Lt => cmp_op(l, r, |ord| ord == std::cmp::Ordering::Less),
        BinaryOp::Lte => cmp_op(l, r, |ord| ord != std::cmp::Ordering::Greater),
        BinaryOp::Gt => cmp_op(l, r, |ord| ord == std::cmp::Ordering::Greater),
        BinaryOp::Gte => cmp_op(l, r, |ord| ord != std::cmp::Ordering::Less),

        // And/Or handled above (short-circuit)
        BinaryOp::And | BinaryOp::Or => unreachable!(),
    }
}

fn num_op(l: &Value, r: &Value, int_op: fn(i64, i64) -> i64, float_op: fn(f64, f64) -> f64) -> Result<Value, String> {
    if let (Some(a), Some(b)) = (l.as_i64(), r.as_i64()) {
        Ok(Value::Number(int_op(a, b).into()))
    } else if let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) {
        Ok(serde_json::Number::from_f64(float_op(a, b)).map(Value::Number).unwrap_or(Value::Null))
    } else {
        Err(format!("cannot apply arithmetic to {:?} and {:?}", l, r))
    }
}

fn cmp_op(l: &Value, r: &Value, pred: fn(std::cmp::Ordering) -> bool) -> Result<Value, String> {
    if let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) {
        Ok(Value::Bool(pred(a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))))
    } else if let (Some(a), Some(b)) = (l.as_str(), r.as_str()) {
        Ok(Value::Bool(pred(a.cmp(b))))
    } else {
        Ok(Value::Bool(false))
    }
}

// ── Function evaluation (V-CEL §3.2.6, §3.2.8) ──

fn eval_function(name: &str, args: &[Expr], vars: &Vars) -> Result<Value, String> {
    match name {
        "size" => {
            if args.len() != 1 { return Err("size() takes 1 argument".into()); }
            let v = eval(&args[0], vars)?;
            Ok(Value::Number(match &v {
                Value::String(s) => s.len() as i64,
                Value::Array(a) => a.len() as i64,
                Value::Object(m) => m.len() as i64,
                _ => 0,
            }.into()))
        }
        "has" => {
            // has(obj.field) — check field existence
            // In V-CEL, `has` takes a field select expression
            if args.len() != 1 { return Err("has() takes 1 argument".into()); }
            match &args[0] {
                Expr::Field(obj, field) => {
                    let v = eval(obj, vars)?;
                    Ok(Value::Bool(match &v {
                        Value::Object(m) => m.contains_key(field),
                        _ => false,
                    }))
                }
                _ => {
                    let v = eval(&args[0], vars)?;
                    Ok(Value::Bool(!v.is_null()))
                }
            }
        }
        "int" => {
            if args.len() != 1 { return Err("int() takes 1 argument".into()); }
            let v = eval(&args[0], vars)?;
            Ok(match &v {
                Value::Number(n) => {
                    let i = n.as_i64().unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64);
                    Value::Number(i.into())
                }
                Value::String(s) => Value::Number(s.parse::<i64>().unwrap_or(0).into()),
                Value::Bool(b) => Value::Number(if *b { 1i64 } else { 0 }.into()),
                _ => Value::Number(0i64.into()),
            })
        }
        "double" | "float" => {
            if args.len() != 1 { return Err(format!("{}() takes 1 argument", name)); }
            let v = eval(&args[0], vars)?;
            let f = match &v {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                Value::String(s) => s.parse().unwrap_or(0.0),
                _ => 0.0,
            };
            Ok(serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null))
        }
        "string" => {
            if args.len() != 1 { return Err("string() takes 1 argument".into()); }
            let v = eval(&args[0], vars)?;
            Ok(Value::String(val_to_string(&v)))
        }
        "type" => {
            if args.len() != 1 { return Err("type() takes 1 argument".into()); }
            let v = eval(&args[0], vars)?;
            Ok(Value::String(match &v {
                Value::Null => "null", Value::Bool(_) => "bool",
                Value::Number(_) => "number", Value::String(_) => "string",
                Value::Array(_) => "list", Value::Object(_) => "map",
            }.into()))
        }
        "max" => {
            if args.is_empty() { return Err("max() needs at least 1 argument".into()); }
            let vals: Result<Vec<Value>, _> = args.iter().map(|a| eval(a, vars)).collect();
            let vals = vals?;
            let mut best = &vals[0];
            for v in &vals[1..] {
                if let (Some(a), Some(b)) = (v.as_f64(), best.as_f64()) {
                    if a > b { best = v; }
                }
            }
            Ok(best.clone())
        }
        "min" => {
            if args.is_empty() { return Err("min() needs at least 1 argument".into()); }
            let vals: Result<Vec<Value>, _> = args.iter().map(|a| eval(a, vars)).collect();
            let vals = vals?;
            let mut best = &vals[0];
            for v in &vals[1..] {
                if let (Some(a), Some(b)) = (v.as_f64(), best.as_f64()) {
                    if a < b { best = v; }
                }
            }
            Ok(best.clone())
        }
        // ── Built-in FaaS functions (feature-gated) ──
        _ => {
            let evaluated_args: Result<Vec<Value>, _> = args.iter().map(|a| eval(a, vars)).collect();
            let evaluated_args = evaluated_args?;
            dispatch_faas(name, &evaluated_args)
        }
    }
}

/// Dispatch to optional FaaS crate functions.
fn dispatch_faas(name: &str, #[allow(unused)] args: &[Value]) -> Result<Value, String> {
    #[cfg(feature = "faas-core")]
    match name {
        "sha256" => return vil_hash::sha256(args),
        "md5" => return vil_hash::md5(args),
        "hmac_sha256" => return vil_hash::hmac_sha256(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "aes_encrypt" => return vil_crypto::aes_encrypt(args),
        "aes_decrypt" => return vil_crypto::aes_decrypt(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "jwt_sign" => return vil_jwt::jwt_sign(args),
        "jwt_verify" => return vil_jwt::jwt_verify(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "uuid_v4" => return vil_id_gen::uuid_v4(args),
        "uuid_v7" => return vil_id_gen::uuid_v7(args),
        "ulid" => return vil_id_gen::ulid(args),
        "nanoid" => return vil_id_gen::nanoid(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "parse_date" => return vil_datefmt::parse_date(args),
        "format_date" => return vil_datefmt::format_date(args),
        "now" => return vil_datefmt::now(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "age" => return vil_duration::age(args),
        "duration" => return vil_duration::duration(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    if name == "parse_csv" { return vil_parse_csv::parse_csv(args); }
    #[cfg(feature = "faas-core")]
    match name {
        "parse_xml" => return vil_parse_xml::parse_xml(args),
        "xpath" => return vil_parse_xml::xpath(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    match name {
        "regex_match" => return vil_regex::regex_match(args),
        "regex_extract" => return vil_regex::regex_extract(args),
        "regex_replace" => return vil_regex::regex_replace(args),
        _ => {}
    }
    #[cfg(feature = "faas-core")]
    if name == "parse_phone" { return vil_phone::parse_phone(args); }

    // ── Batch 2: Transform + Stats + Notification + Geo ──
    #[cfg(feature = "faas-full")]
    if name == "validate_schema" { return vil_validate_schema::validate_schema(args); }
    #[cfg(feature = "faas-full")]
    if name == "mask_pii" { return vil_mask::mask_pii(args); }
    #[cfg(feature = "faas-full")]
    if name == "reshape" { return vil_reshape::reshape(args); }
    #[cfg(feature = "faas-full")]
    if name == "render_template" { return vil_template::render_template(args); }
    #[cfg(feature = "faas-full")]
    if name == "validate_email" { return vil_email_validate::validate_email(args); }
    #[cfg(feature = "faas-full")]
    match name {
        "mean" => return vil_stats::mean(args),
        "median" => return vil_stats::median(args),
        "stdev" => return vil_stats::stdev(args),
        "percentile" => return vil_stats::percentile(args),
        "variance" => return vil_stats::variance(args),
        _ => {}
    }
    #[cfg(feature = "faas-full")]
    if name == "is_anomaly" { return vil_anomaly::is_anomaly(args); }
    #[cfg(feature = "faas-full")]
    if name == "send_email" { return vil_email::send_email(args); }
    #[cfg(feature = "faas-full")]
    if name == "send_webhook" { return vil_webhook_out::send_webhook(args); }
    #[cfg(feature = "faas-full")]
    if name == "geo_distance" { return vil_geodist::geo_distance(args); }

    Err(format!("unknown function: {}(). Enable 'faas-core' or 'faas-full' feature.", name))
}

// ── Method evaluation (V-CEL §3.2.2-4) ──

fn eval_method(obj: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
    match (obj, method) {
        // String methods
        (Value::String(s), "contains") => {
            let arg = args.first().and_then(|a| a.as_str()).unwrap_or("");
            Ok(Value::Bool(s.contains(arg)))
        }
        (Value::String(s), "startsWith") => {
            let arg = args.first().and_then(|a| a.as_str()).unwrap_or("");
            Ok(Value::Bool(s.starts_with(arg)))
        }
        (Value::String(s), "endsWith") => {
            let arg = args.first().and_then(|a| a.as_str()).unwrap_or("");
            Ok(Value::Bool(s.ends_with(arg)))
        }
        (Value::String(s), "size") | (Value::String(s), "length") => Ok(Value::Number((s.len() as i64).into())),
        (Value::String(s), "split") => {
            let delim = args.first().and_then(|a| a.as_str()).unwrap_or("/");
            let parts: Vec<Value> = s.split(delim).map(|p| Value::String(p.to_string())).collect();
            Ok(Value::Array(parts))
        }
        (Value::String(s), "trim") => Ok(Value::String(s.trim().to_string())),
        (Value::String(s), "toUpperCase") => Ok(Value::String(s.to_uppercase())),
        (Value::String(s), "toLowerCase") => Ok(Value::String(s.to_lowercase())),
        (Value::String(s), "replace") => {
            let from = args.first().and_then(|a| a.as_str()).unwrap_or("");
            let to = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
            Ok(Value::String(s.replace(from, to)))
        }
        (Value::String(s), "substring") => {
            let start = args.first().and_then(|a| a.as_u64()).unwrap_or(0) as usize;
            let end = args.get(1).and_then(|a| a.as_u64()).map(|e| e as usize).unwrap_or(s.len());
            Ok(Value::String(s.get(start..end.min(s.len())).unwrap_or("").to_string()))
        }

        // List/Map size method
        (Value::Array(a), "size") | (Value::Array(a), "length") => Ok(Value::Number((a.len() as i64).into())),
        (Value::Array(a), "last") => Ok(a.last().cloned().unwrap_or(Value::Null)),
        (Value::Array(a), "first") => Ok(a.first().cloned().unwrap_or(Value::Null)),
        (Value::Object(m), "size") => Ok(Value::Number((m.len() as i64).into())),

        // Unsupported macros → clear error
        (_, "map" | "filter" | "all" | "exists" | "exists_one") => {
            Err(format!(
                ".{}() is a V-CEL list macro that requires VFlow cloud compiler. \
                 Rewrite using basic expressions or use: vflow compile --cloud",
                method
            ))
        }

        _ => Err(format!("unknown method .{}() on {:?}", method, obj_type_name(obj))),
    }
}

// ── Helpers ──

fn field_access(val: &Value, field: &str) -> Value {
    match val {
        Value::Object(map) => map.get(field).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn val_to_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
    }
}

fn val_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

fn val_eq(a: &Value, b: &Value) -> bool {
    // Numeric equality across int/float
    if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
        return x == y;
    }
    a == b
}

fn obj_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null", Value::Bool(_) => "bool",
        Value::Number(_) => "number", Value::String(_) => "string",
        Value::Array(_) => "list", Value::Object(_) => "map",
    }
}
