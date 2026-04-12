//! MCP protocol handler — stdio JSON-RPC 2.0.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, Write};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    pub fn err(id: Option<Value>, code: i32, message: String) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(JsonRpcError { code, message }) }
    }
}

/// MCP server info returned on initialize.
fn server_info() -> Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "vil-vwfd",
            "version": "0.1.0"
        }
    })
}

fn tools_list() -> Value {
    serde_json::json!([
        {
            "name": "vil_compile",
            "description": "Compile VWFD YAML file. Returns compilation result or errors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "Path to VWFD YAML file" }
                },
                "required": ["file"]
            }
        },
        {
            "name": "vil_lint",
            "description": "Lint VWFD YAML file with VIL Way rules. Returns warnings and errors.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "Path to VWFD YAML file" },
                    "all": { "type": "boolean", "description": "Lint all files in workflows/ directory" }
                }
            }
        },
        {
            "name": "vil_list",
            "description": "List all VWFD workflow files in a directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dir": { "type": "string", "description": "Directory path (default: workflows/)" }
                }
            }
        },
        {
            "name": "vil_explain",
            "description": "Explain a VIL lint rule code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Lint rule code (e.g. VIL-L001)" }
                },
                "required": ["code"]
            }
        },
        {
            "name": "vil_scaffold_workflow",
            "description": "Generate a VWFD workflow stub YAML.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Workflow name" },
                    "route": { "type": "string", "description": "Webhook path (e.g. /api/orders)" },
                    "connectors": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Connector refs to include (e.g. vastar.http, vastar.db.postgres)"
                    }
                },
                "required": ["name", "route"]
            }
        }
    ])
}

fn resources_list() -> Value {
    serde_json::json!([
        {
            "uri": "vil://workflows",
            "name": "Workflow list",
            "description": "List of VWFD workflow files",
            "mimeType": "application/json"
        },
        {
            "uri": "vil://config",
            "name": "Project config",
            "description": "VIL project configuration (.vilrc)",
            "mimeType": "application/json"
        }
    ])
}

/// Handle single JSON-RPC request → response.
pub fn handle_request(req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::ok(req.id.clone(), server_info()),
        "initialized" => JsonRpcResponse::ok(req.id.clone(), serde_json::json!({})),

        "tools/list" => JsonRpcResponse::ok(req.id.clone(), serde_json::json!({ "tools": tools_list() })),
        "tools/call" => {
            let params = req.params.as_ref().cloned().unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
            let result = super::tools::call_tool(tool_name, &arguments);
            JsonRpcResponse::ok(req.id.clone(), result)
        }

        "resources/list" => JsonRpcResponse::ok(req.id.clone(), serde_json::json!({ "resources": resources_list() })),
        "resources/read" => {
            let params = req.params.as_ref().cloned().unwrap_or(Value::Null);
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let result = super::resources::read_resource(uri);
            JsonRpcResponse::ok(req.id.clone(), result)
        }

        _ => JsonRpcResponse::err(req.id.clone(), -32601, format!("method not found: {}", req.method)),
    }
}

/// Run MCP server on stdio. Blocks until stdin closes.
pub fn run_server() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    eprintln!("VIL MCP Server running on stdio");
    eprintln!("Tools: vil_compile, vil_lint, vil_list, vil_explain, vil_scaffold_workflow");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() { continue; }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::err(None, -32700, format!("parse error: {}", e));
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap_or_default());
                let _ = stdout.flush();
                continue;
            }
        };

        let resp = handle_request(&req);
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap_or_default());
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req(method: &str, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: method.into(),
            params,
        }
    }

    #[test]
    fn test_initialize() {
        let resp = handle_request(&make_req("initialize", None));
        assert!(resp.result.is_some());
        assert!(resp.result.as_ref().unwrap()["serverInfo"]["name"] == "vil-vwfd");
    }

    #[test]
    fn test_tools_list() {
        let resp = handle_request(&make_req("tools/list", None));
        let tools = &resp.result.as_ref().unwrap()["tools"];
        assert!(tools.as_array().unwrap().len() >= 5);
    }

    #[test]
    fn test_resources_list() {
        let resp = handle_request(&make_req("resources/list", None));
        let resources = &resp.result.as_ref().unwrap()["resources"];
        assert!(resources.as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_unknown_method() {
        let resp = handle_request(&make_req("nonexistent", None));
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }
}
