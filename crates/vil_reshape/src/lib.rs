use serde_json::{json, Map, Value};

pub fn reshape(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).ok_or("reshape: data required")?;
    let mapping = args
        .get(1)
        .and_then(|v| v.as_object())
        .ok_or("reshape: mapping object required")?;
    let mut result = Map::new();
    for (new_key, source_path) in mapping {
        let path = source_path.as_str().unwrap_or("");
        let val = resolve_path(data, path);
        result.insert(new_key.clone(), val);
    }
    Ok(Value::Object(result))
}

fn resolve_path(data: &Value, path: &str) -> Value {
    let mut current = data;
    for segment in path.split('.') {
        match current {
            Value::Object(m) => current = m.get(segment).unwrap_or(&Value::Null),
            Value::Array(a) => {
                if let Ok(idx) = segment.parse::<usize>() {
                    current = a.get(idx).unwrap_or(&Value::Null);
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }
    current.clone()
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("reshape", reshape)]
}
