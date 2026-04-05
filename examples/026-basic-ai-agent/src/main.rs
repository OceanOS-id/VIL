// ╔════════════════════════════════════════════════════════════╗
// ║  026 — IT Helpdesk Agent (Autonomous Tool Execution)      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   IT Operations — Helpdesk Automation             ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: vil_agent::Agent, ToolRegistry, ReAct loop,    ║
// ║            ServiceCtx, ShmSlice, VilResponse               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Agent autonomously resolves IT tickets using     ║
// ║  tools: system status check, knowledge base search,        ║
// ║  SLA calculator. Multi-turn ReAct loop — LLM decides       ║
// ║  which tools to call, executes them, feeds results back.   ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-basic-ai-agent
// Test:
//   curl -X POST http://localhost:8080/api/helpdesk/ask \
//     -H 'Content-Type: application/json' \
//     -d '{"query":"VPN is not connecting, I need help urgently"}'

use std::sync::Arc;

use async_trait::async_trait;
use vil_agent::tool::{Tool, ToolError, ToolResult};
use vil_agent::Agent;
use vil_llm::{OpenAiConfig, OpenAiProvider};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HelpRequest {
    query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct HelpResponse {
    answer: String,
    tools_used: Vec<String>,
    iterations: usize,
}

struct HelpState {
    agent: Agent,
}

// ── Tool: System Status ──────────────────────────────────────────────────

struct SystemStatusTool;

#[async_trait]
impl Tool for SystemStatusTool {
    fn name(&self) -> &str { "system_status" }
    fn description(&self) -> &str {
        "Check IT service status. Input: {\"service\": \"vpn\"} or {\"service\": \"all\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "service": { "type": "string" } },
            "required": ["service"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let service = params["service"].as_str().unwrap_or("all");
        let services = vec![
            ("vpn", "online", "99.8%"),
            ("email", "online", "99.9%"),
            ("active_directory", "online", "100%"),
            ("dns", "online", "99.99%"),
            ("file_server", "online", "99.5%"),
            ("print_server", "degraded", "95.2%"),
            ("erp", "online", "99.7%"),
            ("wifi", "online", "99.3%"),
        ];
        let output = if service == "all" {
            services.iter()
                .map(|(n, s, u)| format!("{}: {} (uptime {})", n, s, u))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            services.iter()
                .find(|(n, _, _)| *n == service)
                .map(|(n, s, u)| format!("{}: {} (uptime {})", n, s, u))
                .unwrap_or_else(|| format!("{}: unknown service", service))
        };
        Ok(ToolResult { output, metadata: None })
    }
}

// ── Tool: Knowledge Base Search ──────────────────────────────────────────

struct KnowledgeBaseTool;

const KB_ARTICLES: &[(&str, &str, &str)] = &[
    ("KB-001", "Password Reset", "Go to https://password.company.com, click 'Forgot Password', enter your email. Check spam folder. If locked out after 5 attempts, contact IT."),
    ("KB-002", "VPN Setup & Troubleshooting", "Install GlobalProtect from Software Center. Server: vpn.company.com. If not connecting: 1) restart client, 2) check internet, 3) try alternate DNS 8.8.8.8, 4) reinstall client."),
    ("KB-003", "Email Sync Issues", "Outlook: File > Account Settings > Repair. Mobile: remove and re-add account. If calendar not syncing, check delegate permissions."),
    ("KB-004", "Slow Computer", "1) Restart, 2) Check disk space (need >10% free), 3) Close unused browser tabs, 4) Run disk cleanup, 5) If persistent, submit ticket for RAM upgrade."),
    ("KB-005", "Printer Setup", "Add printer via Settings > Printers. Network printer IP: 10.0.1.x. If offline, check physical connection and restart print spooler service."),
    ("KB-006", "WiFi Connection", "SSID: Corp-Secure (802.1x, use AD credentials). Guest: Corp-Guest (open, captive portal). If no connection, forget network and rejoin."),
    ("KB-007", "MFA Setup", "Download Microsoft Authenticator. In portal.company.com > Security > MFA, scan QR code. Backup codes available for emergencies."),
    ("KB-008", "Software Install", "Use Software Center for approved apps. For non-standard software, submit request to IT with business justification. Typical approval: 2-3 days."),
    ("KB-009", "File Sharing", "SharePoint: company.sharepoint.com. OneDrive for personal files. For large files (>25MB), use shared drive \\\\fileserver\\department."),
    ("KB-010", "New Employee Onboarding", "Day 1: AD account + email created. Day 2: VPN + MFA setup. Week 1: department-specific tools. Contact helpdesk@company.com for issues."),
];

#[async_trait]
impl Tool for KnowledgeBaseTool {
    fn name(&self) -> &str { "knowledge_base" }
    fn description(&self) -> &str {
        "Search IT knowledge base articles. Input: {\"query\": \"vpn not connecting\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let query = params["query"].as_str().unwrap_or("").to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(usize, &str, &str, &str)> = KB_ARTICLES.iter()
            .map(|(id, title, content)| {
                let text = format!("{} {}", title.to_lowercase(), content.to_lowercase());
                let score = words.iter().filter(|w| text.contains(*w)).count();
                (score, *id, *title, *content)
            })
            .filter(|(score, _, _, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let top3: Vec<_> = scored.into_iter().take(3).collect();

        let output = if top3.is_empty() {
            "No matching articles found.".into()
        } else {
            top3.iter()
                .map(|(score, id, title, content)| format!("[{}] {} (relevance: {})\n{}", id, title, score, content))
                .collect::<Vec<_>>()
                .join("\n\n")
        };
        Ok(ToolResult { output, metadata: None })
    }
}

// ── Tool: SLA Calculator ─────────────────────────────────────────────────

struct SlaCalculatorTool;

#[async_trait]
impl Tool for SlaCalculatorTool {
    fn name(&self) -> &str { "sla_calculator" }
    fn description(&self) -> &str {
        "Calculate SLA deadline for a ticket. Input: {\"priority\": \"P1\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "priority": { "type": "string", "enum": ["P1","P2","P3","P4"] } },
            "required": ["priority"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let priority = params["priority"].as_str().unwrap_or("P3");
        let (resolve_h, response_min) = match priority {
            "P1" => (4, 15),
            "P2" => (8, 30),
            "P3" => (24, 60),
            "P4" => (72, 240),
            _ => (24, 60),
        };
        let output = format!(
            "Priority: {}\nResolution deadline: {} hours\nFirst response: {} minutes\nEscalation: auto-escalate if no response in {} min",
            priority, resolve_h, response_min, response_min * 2
        );
        Ok(ToolResult { output, metadata: None })
    }
}

// ── Handler ──────────────────────────────────────────────────────────────

async fn ask(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<HelpResponse>> {
    let req: HelpRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let state = ctx.state::<Arc<HelpState>>()
        .map_err(|_| VilError::internal("agent state not found"))?;

    let response = state.agent.run(&req.query).await
        .map_err(|e| VilError::internal(format!("agent failed: {:?}", e)))?;

    Ok(VilResponse::ok(HelpResponse {
        answer: response.answer,
        tools_used: response.tool_calls_made.iter()
            .map(|tc| format!("{}({})", tc.tool, tc.input))
            .collect(),
        iterations: response.iterations,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let llm = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
    ));

    let agent = Agent::builder()
        .llm(llm)
        .tool(Arc::new(SystemStatusTool))
        .tool(Arc::new(KnowledgeBaseTool))
        .tool(Arc::new(SlaCalculatorTool))
        .max_iterations(5)
        .system_prompt(
            "You are an IT helpdesk agent. Use tools to check system status, \
             search knowledge base, and calculate SLA deadlines. \
             Always search the knowledge base before answering user questions."
        )
        .build();

    let state = Arc::new(HelpState { agent });

    let svc = ServiceProcess::new("helpdesk")
        .endpoint(Method::POST, "/ask", post(ask))
        .state(state);

    VilApp::new("it-helpdesk-agent")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
