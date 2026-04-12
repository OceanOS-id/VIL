// 203 — VWFD mode: Code Review with Tool Execution
//
// Workflow: trigger → LLM analysis → tool_executor (NativeCode) → LLM final → respond
//
// The tool_executor runs inline — same process, zero network.
// Parses <tool>name:input</tool> from LLM output, executes calculator/analyzer locally.

use serde_json::{json, Value};

/// Parse <tool>name:input</tool> patterns from LLM output.
fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<tool>") {
        if let Some(end) = remaining[start..].find("</tool>") {
            let inner = &remaining[start + 6..start + end];
            if let Some(colon) = inner.find(':') {
                calls.push((inner[..colon].trim().to_string(), inner[colon + 1..].trim().to_string()));
            }
            remaining = &remaining[start + end + 7..];
        } else {
            break;
        }
    }
    calls
}

/// Execute tool locally: calculator (lines/complexity/math), analyzer (static analysis).
fn execute_tool(name: &str, input: &str) -> String {
    match name {
        "calculator" => {
            let t = input.trim();
            if let Some(rest) = t.strip_prefix("lines(") {
                format!("{} lines", rest.trim_end_matches(')').lines().count())
            } else if let Some(rest) = t.strip_prefix("complexity(") {
                let code = rest.trim_end_matches(')');
                let branches = ["if ", "match ", "while ", "for "].iter()
                    .map(|k| code.matches(k).count()).sum::<usize>()
                    + code.matches("||").count() + code.matches("&&").count();
                format!("cyclomatic_complexity = {}", branches + 1)
            } else {
                let parts: Vec<&str> = t.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    let (a, b) = (parts[0].parse::<f64>().unwrap_or(0.0), parts[2].parse::<f64>().unwrap_or(0.0));
                    format!("{}", match parts[1] { "+" => a+b, "-" => a-b, "*" => a*b, "/" => if b!=0.0 {a/b} else {f64::NAN}, _ => f64::NAN })
                } else { format!("cannot evaluate: {}", t) }
            }
        }
        "analyzer" => {
            let mut f = Vec::new();
            if input.contains("unwrap()") { f.push("WARNING: unwrap() detected"); }
            if input.contains("unsafe") { f.push("WARNING: unsafe block"); }
            if input.contains("clone()") { f.push("NOTE: clone() — consider borrowing"); }
            if input.contains("panic!") || input.contains("todo!") { f.push("WARNING: panic!/todo!"); }
            if !input.contains("///") && !input.contains("//") { f.push("NOTE: no docs"); }
            if f.is_empty() { "No issues found.".into() } else { f.join("\n") }
        }
        _ => format!("Unknown tool: {}", name),
    }
}

/// NativeCode handler: parse LLM output for <tool> calls, execute, return results.
fn tool_executor(input: &Value) -> Result<Value, String> {
    let llm_output = input["llm_output"].as_str().unwrap_or("");
    let code = input["code"].as_str().unwrap_or(llm_output);
    let calls = parse_tool_calls(llm_output);
    let results: Vec<Value> = calls.iter().map(|(name, inp)| {
        let output = execute_tool(name, if name == "analyzer" { code } else { inp });
        json!({"tool": name, "input": inp, "output": output})
    }).collect();
    Ok(json!({"tools": results, "count": results.len()}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/203-llm-code-review-with-tools/vwfd/workflows", 3102)
        .native("tool_executor", tool_executor)
        .run()
        .await;
}
