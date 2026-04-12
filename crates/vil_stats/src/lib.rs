use serde_json::{json, Value};

fn to_f64_vec(args: &[Value]) -> Result<Vec<f64>, String> {
    let arr = args
        .get(0)
        .and_then(|v| v.as_array())
        .ok_or("stats: array required")?;
    arr.iter()
        .map(|v| v.as_f64().ok_or("stats: non-numeric element".to_string()))
        .collect()
}

pub fn mean(args: &[Value]) -> Result<Value, String> {
    let v = to_f64_vec(args)?;
    if v.is_empty() {
        return Ok(Value::Null);
    }
    let sum: f64 = v.iter().sum();
    Ok(json!(sum / v.len() as f64))
}

pub fn median(args: &[Value]) -> Result<Value, String> {
    let mut v = to_f64_vec(args)?;
    if v.is_empty() {
        return Ok(Value::Null);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    if v.len() % 2 == 0 {
        Ok(json!((v[mid - 1] + v[mid]) / 2.0))
    } else {
        Ok(json!(v[mid]))
    }
}

pub fn stdev(args: &[Value]) -> Result<Value, String> {
    let v = to_f64_vec(args)?;
    if v.len() < 2 {
        return Ok(json!(0.0));
    }
    let mean_val: f64 = v.iter().sum::<f64>() / v.len() as f64;
    let variance: f64 =
        v.iter().map(|x| (x - mean_val).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
    Ok(json!(variance.sqrt()))
}

pub fn percentile(args: &[Value]) -> Result<Value, String> {
    let mut v = to_f64_vec(args)?;
    let p = args
        .get(1)
        .and_then(|v| v.as_f64())
        .ok_or("percentile: p required (0-100)")?;
    if v.is_empty() {
        return Ok(Value::Null);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((p / 100.0) * (v.len() - 1) as f64).round() as usize;
    Ok(json!(v[idx.min(v.len() - 1)]))
}

pub fn variance(args: &[Value]) -> Result<Value, String> {
    let v = to_f64_vec(args)?;
    if v.len() < 2 {
        return Ok(json!(0.0));
    }
    let mean_val: f64 = v.iter().sum::<f64>() / v.len() as f64;
    let var: f64 =
        v.iter().map(|x| (x - mean_val).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
    Ok(json!(var))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![
        ("mean", mean),
        ("median", median),
        ("stdev", stdev),
        ("percentile", percentile),
        ("variance", variance),
    ]
}
