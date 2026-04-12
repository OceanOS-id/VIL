use serde_json::{json, Value};
use regex::Regex;

pub fn regex_match(args: &[Value]) -> Result<Value, String> {
    let text = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("regex_match: text required")?;
    let pattern = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("regex_match: pattern required")?;
    let re = Regex::new(pattern).map_err(|e| format!("regex_match: {}", e))?;
    Ok(Value::Bool(re.is_match(text)))
}

pub fn regex_extract(args: &[Value]) -> Result<Value, String> {
    let text = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("regex_extract: text required")?;
    let pattern = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("regex_extract: pattern required")?;
    let re = Regex::new(pattern).map_err(|e| format!("regex_extract: {}", e))?;
    if let Some(caps) = re.captures(text) {
        let groups: Vec<Value> = caps
            .iter()
            .map(|m| {
                m.map(|m| Value::String(m.as_str().to_string()))
                    .unwrap_or(Value::Null)
            })
            .collect();
        Ok(json!(groups))
    } else {
        Ok(Value::Array(vec![]))
    }
}

pub fn regex_replace(args: &[Value]) -> Result<Value, String> {
    let text = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("regex_replace: text required")?;
    let pattern = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("regex_replace: pattern required")?;
    let replacement = args
        .get(2)
        .and_then(|v| v.as_str())
        .ok_or("regex_replace: replacement required")?;
    let re = Regex::new(pattern).map_err(|e| format!("regex_replace: {}", e))?;
    Ok(Value::String(re.replace_all(text, replacement).into_owned()))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![
        ("regex_match", regex_match),
        ("regex_extract", regex_extract),
        ("regex_replace", regex_replace),
    ]
}
