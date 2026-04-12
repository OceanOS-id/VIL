use serde_json::{json, Value};
fn code_file_tools(input: &Value) -> Result<Value, String> {
    let files = vec![("main.rs", "fn main() { let x = vec![1,2,3]; println!(\"{:?}\", x.clone()); }"), ("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }")];
    let results: Vec<Value> = files.iter().map(|(name, content)| {
        let lines = content.lines().count();
        let has_clone = content.contains("clone()");
        let has_unwrap = content.contains("unwrap()");
        json!({"file": name, "lines": lines, "issues": {"clone": has_clone, "unwrap": has_unwrap}})
    }).collect();
    Ok(json!({"files_analyzed": results.len(), "results": results}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/403-agent-code-file-reviewer/vwfd/workflows", 3122)
        .native("code_file_tools", code_file_tools).run().await;
}
