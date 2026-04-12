use serde_json::{json, Value};
use quick_xml::Reader;
use quick_xml::events::Event;

pub fn parse_xml(args: &[Value]) -> Result<Value, String> {
    let data = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("parse_xml: XML string required")?;
    let mut reader = Reader::from_str(data);
    let mut elements = Vec::new();
    let mut buf = Vec::new();
    let mut current_path: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs: serde_json::Map<String, Value> = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&a.value).to_string();
                        (key, Value::String(val))
                    })
                    .collect();
                current_path.push(name.clone());
                if !attrs.is_empty() {
                    elements.push(json!({
                        "tag": name,
                        "path": current_path.join("/"),
                        "attributes": attrs
                    }));
                }
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|e| format!("parse_xml: {}", e))?
                    .to_string();
                let text = text.trim().to_string();
                if !text.is_empty() && !current_path.is_empty() {
                    let tag = current_path.last().unwrap().clone();
                    elements.push(json!({
                        "tag": tag,
                        "path": current_path.join("/"),
                        "text": text
                    }));
                }
            }
            Ok(Event::End(_)) => {
                current_path.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("parse_xml: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    let count = elements.len();
    Ok(json!({"elements": elements, "count": count}))
}

pub fn xpath(args: &[Value]) -> Result<Value, String> {
    let parsed = args.get(0).ok_or("xpath: parsed XML required")?;
    let expr = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("xpath: expression required")?;
    let elements = parsed
        .get("elements")
        .and_then(|v| v.as_array())
        .ok_or("xpath: elements array required")?;

    let matches: Vec<&Value> = elements
        .iter()
        .filter(|el| {
            let path = el.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let tag = el.get("tag").and_then(|v| v.as_str()).unwrap_or("");
            path.contains(expr) || tag == expr
        })
        .collect();

    if matches.len() == 1 {
        if let Some(text) = matches[0].get("text") {
            return Ok(text.clone());
        }
        return Ok(matches[0].clone());
    }
    Ok(json!(matches))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("parse_xml", parse_xml), ("xpath", xpath)]
}
