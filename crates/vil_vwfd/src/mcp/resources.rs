//! MCP resource implementations.

use serde_json::Value;

/// Read MCP resource by URI.
pub fn read_resource(uri: &str) -> Value {
    match uri {
        "vil://workflows" => read_workflows(),
        "vil://config" => read_config(),
        uri if uri.starts_with("vil://workflows/") => {
            let name = &uri["vil://workflows/".len()..];
            read_workflow_file(name)
        }
        _ => serde_json::json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": format!("unknown resource: {}", uri)
            }]
        }),
    }
}

fn read_workflows() -> Value {
    let result = crate::loader::load_dir("workflows");
    let list: Vec<Value> = result.graphs.iter().map(|g| {
        serde_json::json!({
            "id": g.id,
            "route": g.webhook_route,
            "trigger": g.trigger_type,
            "nodes": g.node_count(),
        })
    }).collect();

    serde_json::json!({
        "contents": [{
            "uri": "vil://workflows",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&list).unwrap_or_default()
        }]
    })
}

fn read_workflow_file(name: &str) -> Value {
    // Try workflows/{name}.yaml, .yml, .vwfd
    for ext in &["yaml", "yml", "vwfd"] {
        let path = format!("workflows/{}.{}", name, ext);
        if let Ok(content) = std::fs::read_to_string(&path) {
            return serde_json::json!({
                "contents": [{
                    "uri": format!("vil://workflows/{}", name),
                    "mimeType": "text/yaml",
                    "text": content
                }]
            });
        }
    }

    serde_json::json!({
        "contents": [{
            "uri": format!("vil://workflows/{}", name),
            "mimeType": "text/plain",
            "text": format!("workflow '{}' not found in workflows/", name)
        }]
    })
}

fn read_config() -> Value {
    let config = if let Ok(content) = std::fs::read_to_string(".vilrc") {
        content
    } else if let Ok(content) = std::fs::read_to_string("vil.toml") {
        content
    } else {
        "# No .vilrc or vil.toml found\n# Create one to configure VIL project".into()
    };

    serde_json::json!({
        "contents": [{
            "uri": "vil://config",
            "mimeType": "text/plain",
            "text": config
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config_no_file() {
        let result = read_resource("vil://config");
        let text = result["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("No .vilrc") || text.contains("["));
    }

    #[test]
    fn test_read_unknown() {
        let result = read_resource("vil://nonexistent");
        let text = result["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown resource"));
    }
}
