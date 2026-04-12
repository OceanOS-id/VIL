use serde_json::Value;
use handlebars::Handlebars;

pub fn render_template(args: &[Value]) -> Result<Value, String> {
    let template = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("render_template: template string required")?;
    let data = args.get(1).unwrap_or(&Value::Null);
    let reg = Handlebars::new();
    let rendered = reg
        .render_template(template, data)
        .map_err(|e| format!("render_template: {}", e))?;
    Ok(Value::String(rendered))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("render_template", render_template)]
}
