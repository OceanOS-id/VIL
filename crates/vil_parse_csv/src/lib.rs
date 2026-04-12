use serde_json::{json, Value};

pub fn parse_csv(args: &[Value]) -> Result<Value, String> {
    let data = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("parse_csv: data string required")?;
    let delimiter = args.get(1).and_then(|v| v.as_str()).unwrap_or(",");
    let has_header = args.get(2).and_then(|v| v.as_bool()).unwrap_or(true);

    let delim_byte = delimiter.as_bytes().first().copied().unwrap_or(b',');
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim_byte)
        .has_headers(has_header)
        .from_reader(data.as_bytes());

    let headers: Vec<String> = if has_header {
        rdr.headers()
            .map_err(|e| format!("parse_csv: {}", e))?
            .iter()
            .map(|h| h.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("parse_csv: {}", e))?;
        if has_header && !headers.is_empty() {
            let mut obj = serde_json::Map::new();
            for (i, field) in record.iter().enumerate() {
                let key = headers
                    .get(i)
                    .map(|h| h.as_str())
                    .unwrap_or("unknown");
                obj.insert(key.to_string(), Value::String(field.to_string()));
            }
            rows.push(Value::Object(obj));
        } else {
            let arr: Vec<Value> = record
                .iter()
                .map(|f| Value::String(f.to_string()))
                .collect();
            rows.push(Value::Array(arr));
        }
    }

    let count = rows.len();
    Ok(json!({"rows": rows, "count": count, "headers": headers}))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("parse_csv", parse_csv)]
}
