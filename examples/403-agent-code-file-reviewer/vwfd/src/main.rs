// 403 — Code File Review Agent (VWFD)
// Business logic identical to standard:
//   POST /api/code-review — eager tool execution (read_file, count_lines, find_pattern)
//   → LLM review → { content, tools_executed: [{tool, input, output, success}], files_available }
use serde_json::{json, Value};

fn code_file_tools(input: &Value) -> Result<Value, String> {
    let mock_files = vec![
        ("src/main.rs", "fn main() {\n    let data = fetch_data().unwrap();\n    let processed = data.iter().map(|x| x.clone()).collect::<Vec<_>>();\n    // TODO: handle errors properly\n    println!(\"{:?}\", processed);\n}\n\nfn fetch_data() -> Result<Vec<String>, Box<dyn std::error::Error>> {\n    Ok(vec![\"hello\".into()])\n}"),
        ("src/handler.rs", "use std::sync::Arc;\n\npub async fn handle_request(data: Arc<Vec<u8>>) -> Result<String, String> {\n    let text = String::from_utf8(data.to_vec()).unwrap();\n    unsafe { std::mem::transmute::<&str, &str>(&text) };\n    Ok(text)\n}"),
        ("src/lib.rs", "pub mod handler;\n\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn test_add() { assert_eq!(add(2, 3), 5); }\n}"),
    ];

    let patterns = input["patterns"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["unwrap".into(), "unsafe".into(), "clone".into(), "TODO".into()]);

    let mut results: Vec<Value> = Vec::new();

    for (path, content) in &mock_files {
        let lines = content.lines().count();
        let blank = content.lines().filter(|l| l.trim().is_empty()).count();
        let comments = content.lines().filter(|l| l.trim().starts_with("//")).count();

        results.push(json!({
            "tool": "read_file",
            "input": path,
            "output": format!("{} ({} lines)", path, lines),
            "success": true
        }));
        results.push(json!({
            "tool": "count_lines",
            "input": path,
            "output": format!("total: {}, blank: {}, comments: {}", lines, blank, comments),
            "success": true
        }));

        for pattern in &patterns {
            let matches: Vec<usize> = content.lines().enumerate()
                .filter(|(_, l)| l.contains(pattern.as_str()))
                .map(|(i, _)| i + 1)
                .collect();
            if !matches.is_empty() {
                results.push(json!({
                    "tool": "find_pattern",
                    "input": format!("{}:{}", path, pattern),
                    "output": format!("found at lines {:?}", matches),
                    "success": true
                }));
            }
        }
    }

    Ok(json!({
        "results": results,
        "files": mock_files.iter().map(|(p, _)| json!({"path": p, "lines": p.len()})).collect::<Vec<_>>()
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/403-agent-code-file-reviewer/vwfd/workflows", 3122)
        .native("code_file_tools", code_file_tools)
        .run().await;
}
