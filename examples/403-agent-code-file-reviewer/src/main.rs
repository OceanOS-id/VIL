// NOTE: Uses in-memory mock filesystem for demo. Production: wire to real fs.
// ╔════════════════════════════════════════════════════════════╗
// ║  403 — DevOps Incident Responder                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                        ║
// ║  Token:    N/A                                           ║
// ║  Unique:   FILE SYSTEM TOOLS — agent reads files, counts ║
// ║            lines, searches patterns. Code-focused tools  ║
// ║            with mock file system. Different tool set     ║
// ║            from 402 (no HTTP, file I/O instead).         ║
// ║  Domain:   Agent reads logs, searches patterns, counts   ║
// ║            metrics — automated incident triage            ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p agent-plugin-usage-code-review
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Review main.rs — check for unwrap() usage and count lines"}' \
//     http://localhost:3122/api/code-review
//
// BUSINESS CONTEXT:
//   DevOps incident responder agent. When an on-call engineer triggers the
//   agent, it automatically: (1) reads source files to understand context,
//   (2) counts lines to assess scope, and (3) searches for known anti-patterns
//   (unwrap, TODO, unsafe) that correlate with production incidents. The agent
//   produces a triage report with severity assessment, reducing MTTR (Mean
//   Time To Resolution) by automating the initial investigation phase.
//
// HOW THIS DIFFERS FROM 402:
//   402 = HTTP fetch tool (REST API data)
//   403 = File system tools (read_file, count_lines, find_pattern)
//   Completely different tool set and domain (code files vs products).

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CodeReviewAgentState {
    pub files_reviewed: u64,
    pub patterns_searched: u64,
    pub total_issues_found: u64,
}

#[derive(Clone, Debug)]
pub struct FileToolEvent {
    pub tool: String,
    pub file_path: String,
    pub result_summary: String,
}

#[vil_fault]
pub enum CodeReviewAgentFault {
    FileNotFound,
    PatternInvalid,
    FileTooLarge,
    LlmUpstreamError,
}

// ── Mock File System ────────────────────────────────────────────────
// Simulates a project's source tree. In a real DevOps incident responder,
// this would read from the actual filesystem or a git repo checkout.
// The mock files contain intentional issues (unwrap, TODO) that the
// agent should detect and report.

struct MockFile {
    path: &'static str,
    content: &'static str,
}

const MOCK_FILES: &[MockFile] = &[
    MockFile {
        path: "src/main.rs",
        content: r#"use std::collections::HashMap;

fn main() {
    let config = load_config("config.toml").unwrap();
    let db = connect_db(&config.db_url).unwrap();

    let mut cache: HashMap<String, String> = HashMap::new();

    for item in db.query("SELECT * FROM users").unwrap() {
        let name = item.get("name").unwrap().clone();
        let email = item.get("email").unwrap().clone();
        cache.insert(name, email);
    }

    println!("Loaded {} users", cache.len());

    // TODO: Add error handling
    let server = start_server(8080).unwrap();
    server.run().unwrap();
}

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn connect_db(url: &str) -> Result<Database, Box<dyn std::error::Error>> {
    Database::connect(url)
}"#,
    },
    MockFile {
        path: "src/handler.rs",
        content: r#"use axum::{Json, extract::State};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub id: u64,
    pub name: String,
    pub email: String,
}

/// Get user by ID
pub async fn get_user(
    State(db): State<Database>,
    body: ShmSlice,
) -> Result<Json<UserResponse>, AppError> {
    let req: UserRequest = body.json().expect("invalid JSON body");
    let user = db.find_user(req.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    match user {
        Some(u) => Ok(Json(UserResponse {
            id: u.id,
            name: u.name,
            email: u.email,
        })),
        None => Err(AppError::NotFound("User not found".into())),
    }
}

/// Create new user
pub async fn create_user(
    State(db): State<Database>,
    body: ShmSlice,
) -> Result<Json<UserResponse>, AppError> {
    let req: CreateUserRequest = body.json().expect("invalid JSON body");
    // Validate email
    if !req.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email".into()));
    }

    let user = db.create_user(&req.name, &req.email)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(UserResponse {
        id: user.id,
        name: user.name,
        email: user.email,
    }))
}"#,
    },
    MockFile {
        path: "src/lib.rs",
        content: r#"pub mod handler;
pub mod config;
pub mod error;

pub use config::Config;
pub use error::AppError;"#,
    },
];

// ── Tool Implementations ────────────────────────────────────────────
// Three file-oriented tools for code investigation:
//   read_file     — retrieve source content (needed for full context)
//   count_lines   — quick complexity proxy (large files = more risk)
//   find_pattern  — search for known anti-patterns (unwrap, TODO, unsafe)

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ToolResult {
    tool: String,
    input: String,
    output: String,
    success: bool,
}

fn tool_read_file(path: &str) -> ToolResult {
    if let Some(file) = MOCK_FILES.iter().find(|f| f.path == path) {
        ToolResult {
            tool: "read_file".into(),
            input: path.into(),
            output: file.content.to_string(),
            success: true,
        }
    } else {
        let available: Vec<&str> = MOCK_FILES.iter().map(|f| f.path).collect();
        ToolResult {
            tool: "read_file".into(),
            input: path.into(),
            output: format!("File not found: {}. Available: {:?}", path, available),
            success: false,
        }
    }
}

fn tool_count_lines(path: &str) -> ToolResult {
    if let Some(file) = MOCK_FILES.iter().find(|f| f.path == path) {
        let total = file.content.lines().count();
        let blank = file.content.lines().filter(|l| l.trim().is_empty()).count();
        let code = total - blank;
        ToolResult {
            tool: "count_lines".into(),
            input: path.into(),
            output: format!("total: {}, code: {}, blank: {}", total, code, blank),
            success: true,
        }
    } else {
        ToolResult {
            tool: "count_lines".into(),
            input: path.into(),
            output: format!("File not found: {}", path),
            success: false,
        }
    }
}

fn tool_find_pattern(path: &str, pattern: &str) -> ToolResult {
    if let Some(file) = MOCK_FILES.iter().find(|f| f.path == path) {
        let matches: Vec<String> = file
            .content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(pattern))
            .map(|(i, line)| format!("  L{}: {}", i + 1, line.trim()))
            .collect();

        let output = if matches.is_empty() {
            format!("No matches for '{}' in {}", pattern, path)
        } else {
            format!(
                "{} matches for '{}' in {}:\n{}",
                matches.len(),
                pattern,
                path,
                matches.join("\n")
            )
        };

        ToolResult {
            tool: "find_pattern".into(),
            input: format!("{}:{}", path, pattern),
            output,
            success: true,
        }
    } else {
        ToolResult {
            tool: "find_pattern".into(),
            input: format!("{}:{}", path, pattern),
            output: format!("File not found: {}", path),
            success: false,
        }
    }
}

// ── Request / Response ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CodeReviewRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct CodeReviewResponse {
    content: String,
    tools_executed: Vec<ToolResult>,
    files_available: Vec<String>,
}

// ── Handler ─────────────────────────────────────────────────────────

async fn code_review_handler(body: ShmSlice) -> HandlerResult<VilResponse<CodeReviewResponse>> {
    let req: CodeReviewRequest = body.json().expect("invalid JSON body");
    // Step 1: Pre-execute tools based on query.
    // Business strategy: eagerly gather ALL available data before calling LLM.
    // This reduces LLM round-trips (cheaper, faster) vs. letting the LLM
    // decide which files to read one at a time.
    let mut tool_results = Vec::new();

    // Always read the files mentioned in the prompt, or all files
    let files_to_read: Vec<&str> = MOCK_FILES
        .iter()
        .filter(|f| {
            req.prompt.to_lowercase().contains(&f.path.to_lowercase())
                || req.prompt.contains("all")
                || req.prompt.contains("review")
        })
        .map(|f| f.path)
        .collect();

    let files_to_read = if files_to_read.is_empty() {
        vec![MOCK_FILES[0].path] // Default to first file
    } else {
        files_to_read
    };

    for path in &files_to_read {
        // Read file
        tool_results.push(tool_read_file(path));
        // Count lines
        tool_results.push(tool_count_lines(path));
        // Search for common issues — these patterns correlate with
        // production incidents: unwrap() panics, TODO = incomplete logic,
        // unsafe = memory safety risks, clone() = performance concerns.
        tool_results.push(tool_find_pattern(path, "unwrap()"));
        tool_results.push(tool_find_pattern(path, "TODO"));
        tool_results.push(tool_find_pattern(path, "unsafe"));
        tool_results.push(tool_find_pattern(path, "clone()"));
    }

    // Step 2: Build context from tool results for LLM
    let tool_context: String = tool_results
        .iter()
        .map(|r| format!("[{}] {}\n{}", r.tool, r.input, r.output))
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = format!(
        "You are a code review agent. The following tool results contain file contents \
         and analysis from the project. Provide a thorough code review including:\n\
         1. Summary of each file\n\
         2. Issues found (unwrap usage, TODO items, potential bugs)\n\
         3. Improvement suggestions\n\
         4. Overall quality rating (1-5 stars)\n\n\
         Tool Results:\n{}",
        tool_context
    );

    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic anchors
    let _event = std::any::type_name::<AgentCompletionEvent>();
    let _fault = std::any::type_name::<AgentFault>();
    let _state = std::any::type_name::<AgentMemoryState>();

    Ok(VilResponse::ok(CodeReviewResponse {
        content,
        tools_executed: tool_results,
        files_available: MOCK_FILES.iter().map(|f| f.path.to_string()).collect(),
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  403 — Agent Code File Reviewer (VilApp)                   ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: File system tools — read, count, pattern search   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Tools:");
    println!("    - read_file   : read source file content");
    println!("    - count_lines : count total/code/blank lines");
    println!("    - find_pattern: regex search in code");
    println!(
        "  Mock files: {:?}",
        MOCK_FILES.iter().map(|f| f.path).collect::<Vec<_>>()
    );
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    println!("  Listening on http://localhost:3122/api/code-review");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("code-review-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/code-review", post(code_review_handler))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    VilApp::new("code-file-reviewer-agent")
        .port(3122)
        .service(svc)
        .run()
        .await;
}
