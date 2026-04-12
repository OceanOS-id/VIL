use serde_json::{json, Value};

pub fn validate_email(args: &[Value]) -> Result<Value, String> {
    let email = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("validate_email: email required")?;
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Ok(json!({"valid": false, "error": "invalid format"}));
    }
    let domain = parts[1];
    let has_dot = domain.contains('.');
    let valid_chars = domain
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-');
    let valid = has_dot && valid_chars && !domain.starts_with('.') && !domain.ends_with('.');
    Ok(json!({"valid": valid, "local": parts[0], "domain": domain}))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("validate_email", validate_email)]
}
