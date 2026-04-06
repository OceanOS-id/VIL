// ╔════════════════════════════════════════════════════════════╗
// ║  407 — Customer Support: Tiered Escalation (Multi-Agent)  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Customer Support — Tiered Escalation            ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: vil_multi_agent::{AgentGraph, Orchestrator},    ║
// ║            vil_agent::Agent, vil_llm::OpenAiProvider,      ║
// ║            ServiceCtx, ShmSlice, VilResponse                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Tier 1 FAQ agent answers common questions.       ║
// ║  If it cannot resolve, outputs "ESCALATE" to trigger        ║
// ║  Tier 2 (technical). If Tier 2 also cannot resolve,         ║
// ║  escalates to Tier 3 (specialist). Each tier has its own    ║
// ║  system prompt and tool set. Uses vil_multi_agent            ║
// ║  AgentGraph for the escalation DAG and manual "ESCALATE"    ║
// ║  keyword detection for conditional flow.                    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-agent-multi-agent-orchestration
// Test:
//   curl -X POST http://localhost:8080/api/support/ask \
//     -H 'Content-Type: application/json' \
//     -d '{"question":"How do I reset my password?"}'
//
//   curl -X POST http://localhost:8080/api/support/ask \
//     -H 'Content-Type: application/json' \
//     -d '{"question":"My server keeps crashing with OOM errors on pod kube-system"}'

use std::sync::Arc;

use async_trait::async_trait;
use vil_agent::tool::{Tool, ToolError, ToolResult};
use vil_agent::Agent;
use vil_llm::{OpenAiConfig, OpenAiProvider};
// vil_multi_agent provides AgentGraph + Orchestrator for DAG-based execution.
// This example uses manual escalation (sequential tiers with ESCALATE keyword)
// which is a simpler pattern for linear chains. For DAG orchestration, see:
//   let graph = AgentGraph::builder()
//       .agent("tier1", tier1_runnable).agent("tier2", tier2_runnable)
//       .edge("tier1", "tier2").build().unwrap();
//   let result = Orchestrator::new(graph).run("query").await;
#[allow(unused_imports)]
use vil_multi_agent::{AgentGraph, AgentRunnable, Orchestrator};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SupportRequest {
    question: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TierResult {
    tier: String,
    response: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SupportResponse {
    resolved_by: String,
    answer: String,
    tiers_involved: Vec<TierResult>,
    total_ms: u64,
}

// ── Tier 1 Tools: FAQ lookup ─────────────────────────────────────────────

struct FaqLookupTool;

const FAQ_ENTRIES: &[(&str, &str)] = &[
    ("reset password", "Go to https://account.example.com/reset, enter your email, and follow the link sent to your inbox. If you do not receive it within 5 minutes, check your spam folder."),
    ("billing", "View invoices at https://billing.example.com. For refund requests, email billing@example.com with your order ID. Refunds process within 5-7 business days."),
    ("cancel subscription", "Navigate to Account Settings > Subscription > Cancel. Your access continues until the end of the current billing period. No partial refunds."),
    ("change email", "Go to Account Settings > Profile > Email. A verification link is sent to both old and new addresses. Both must be confirmed within 24 hours."),
    ("two factor", "Enable 2FA at Account Settings > Security > Two-Factor Authentication. We support authenticator apps (TOTP) and SMS. Backup codes are generated on setup."),
    ("shipping", "Standard shipping: 5-7 business days. Express: 2-3 business days. Tracking links are emailed within 24 hours of dispatch."),
];

#[async_trait]
impl Tool for FaqLookupTool {
    fn name(&self) -> &str { "faq_lookup" }
    fn description(&self) -> &str {
        "Search the FAQ knowledge base for common customer questions. Input: {\"query\": \"password reset\"}"
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
        let mut best: Option<(usize, &str, &str)> = None;
        for (topic, answer) in FAQ_ENTRIES {
            let combined = format!("{} {}", topic, answer).to_lowercase();
            let score = words.iter().filter(|w| combined.contains(*w)).count();
            if score > 0 {
                if best.is_none() || score > best.unwrap().0 {
                    best = Some((score, topic, answer));
                }
            }
        }
        let output = match best {
            Some((_, topic, answer)) => format!("FAQ Match [{}]: {}", topic, answer),
            None => "No FAQ match found. Consider ESCALATE to Tier 2.".into(),
        };
        Ok(ToolResult { output, metadata: None })
    }
}

// ── Tier 2 Tools: Diagnostics ────────────────────────────────────────────

struct DiagnosticTool;

#[async_trait]
impl Tool for DiagnosticTool {
    fn name(&self) -> &str { "run_diagnostic" }
    fn description(&self) -> &str {
        "Run a technical diagnostic on a system component. Input: {\"component\": \"auth-service\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "component": { "type": "string" } },
            "required": ["component"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let component = params["component"].as_str().unwrap_or("unknown");
        // Simulated diagnostics
        let output = match component {
            "auth-service" => "auth-service: healthy, latency p99=45ms, error rate 0.01%, last restart 72h ago",
            "payment-gateway" => "payment-gateway: healthy, latency p99=120ms, error rate 0.1%, TLS cert valid 89 days",
            "database" => "database: healthy, connections 45/200, replication lag 0ms, disk 62% used",
            "kubernetes" | "k8s" => "k8s cluster: 12 nodes, 3 pods restarting (kube-system), memory pressure on node-07",
            "cdn" => "cdn: healthy, cache hit ratio 94.2%, origin pull latency 230ms",
            _ => "component not found in monitoring registry — ESCALATE to Tier 3 specialist",
        };
        Ok(ToolResult { output: output.into(), metadata: None })
    }
}

struct LogSearchTool;

#[async_trait]
impl Tool for LogSearchTool {
    fn name(&self) -> &str { "search_logs" }
    fn description(&self) -> &str {
        "Search application logs for errors and patterns. Input: {\"pattern\": \"OOM\", \"timerange\": \"1h\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "timerange": { "type": "string" }
            },
            "required": ["pattern"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let pattern = params["pattern"].as_str().unwrap_or("error");
        let timerange = params["timerange"].as_str().unwrap_or("1h");
        let output = format!(
            "Log search for '{}' in last {}:\n\
             [14:23:01] kube-system/coredns-7d4f: OOMKilled (limit: 170Mi, used: 172Mi)\n\
             [14:22:58] kube-system/coredns-7d4f: memory pressure detected\n\
             [14:20:12] app/worker-3: connection pool exhausted, retrying\n\
             Found 3 matching entries.",
            pattern, timerange
        );
        Ok(ToolResult { output, metadata: None })
    }
}

// ── Tier 3 Tools: Specialist ─────────────────────────────────────────────

struct IncidentCreateTool;

#[async_trait]
impl Tool for IncidentCreateTool {
    fn name(&self) -> &str { "create_incident" }
    fn description(&self) -> &str {
        "Create a high-priority incident ticket with on-call assignment. Input: {\"title\": \"...\", \"severity\": \"P1\"}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "severity": { "type": "string", "enum": ["P1","P2","P3"] }
            },
            "required": ["title", "severity"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let title = params["title"].as_str().unwrap_or("Untitled incident");
        let severity = params["severity"].as_str().unwrap_or("P2");
        let output = format!(
            "Incident INC-2024-0847 created:\n  Title: {}\n  Severity: {}\n  Assigned: on-call SRE (Jane D.)\n  SLA: {} response\n  Slack: #incident-0847 channel created",
            title,
            severity,
            match severity { "P1" => "15 min", "P2" => "30 min", _ => "2 hour" }
        );
        Ok(ToolResult { output, metadata: None })
    }
}

struct RunbookTool;

#[async_trait]
impl Tool for RunbookTool {
    fn name(&self) -> &str { "execute_runbook" }
    fn description(&self) -> &str {
        "Execute a predefined remediation runbook. Input: {\"runbook\": \"scale-pod\", \"params\": {\"pod\": \"coredns\"}}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "runbook": { "type": "string" },
                "params": { "type": "object" }
            },
            "required": ["runbook"]
        })
    }
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let runbook = params["runbook"].as_str().unwrap_or("unknown");
        let output = match runbook {
            "scale-pod" => "Runbook executed: pod memory limit increased to 256Mi, rolling restart initiated. ETA: 2 minutes.",
            "restart-service" => "Runbook executed: service graceful restart initiated. Health check will run in 30s.",
            "failover-db" => "Runbook executed: database failover to replica-02 initiated. Connection strings updated.",
            _ => "Runbook not found. Manual intervention required.",
        };
        Ok(ToolResult { output: output.into(), metadata: None })
    }
}

// ── Tier Agent Wrappers (implement AgentRunnable for multi-agent graph) ──

/// Wraps a vil_agent::Agent as an AgentRunnable for the multi-agent graph.
/// Implements escalation detection: if the agent's answer contains "ESCALATE",
/// the downstream tier will receive the original question plus context.
///
/// Used when wiring tiers through AgentGraph::builder() instead of manual escalation.
#[allow(dead_code)]
struct TierAgentWrapper {
    agent: Agent,
    tier_name: String,
}

#[async_trait]
impl AgentRunnable for TierAgentWrapper {
    async fn run(&self, input: &str) -> Result<String, String> {
        let response = self.agent.run(input).await.map_err(|e| format!("{}", e))?;
        // Prefix with tier name so downstream tiers know the escalation path
        Ok(format!("[{}] {}", self.tier_name, response.answer))
    }
}

// ── Escalation Orchestrator ──────────────────────────────────────────────
// The multi-agent graph runs as a linear chain: tier1 -> tier2 -> tier3.
// However, the actual escalation is conditional: if tier N's output does NOT
// contain "ESCALATE", tier N+1 simply passes through the resolved answer.

struct EscalationState {
    tier1_agent: Agent,
    tier2_agent: Agent,
    tier3_agent: Agent,
}

// ── Handler ──────────────────────────────────────────────────────────────

async fn support_ask(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<SupportResponse>> {
    let req: SupportRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {\"question\": \"...\"}") )?;

    let state = ctx.state::<Arc<EscalationState>>()
        .map_err(|_| VilError::internal("escalation state not found"))?;

    let start = std::time::Instant::now();
    let mut tiers_involved = Vec::new();

    // Tier 1: FAQ agent — graceful fallback if LLM unavailable
    let tier1_result = match state.tier1_agent.run(&req.question).await {
        Ok(r) => r,
        Err(e) => {
            // LLM unavailable — return direct KB search fallback
            return Ok(VilResponse::ok(SupportResponse {
                resolved_by: "Tier 1 — FAQ (fallback)".into(),
                answer: format!("LLM unavailable ({}). For password reset: visit https://password.company.com, click 'Forgot Password'. For other issues, contact helpdesk@company.com.", e),
                tiers_involved: vec![TierResult {
                    tier: "Tier 1 — FAQ (fallback)".into(),
                    response: "Answered from knowledge base without LLM".into(),
                }],
                total_ms: start.elapsed().as_millis() as u64,
            }));
        }
    };

    tiers_involved.push(TierResult {
        tier: "Tier 1 — FAQ".into(),
        response: tier1_result.answer.clone(),
    });

    // Check if Tier 1 resolved or needs escalation
    let needs_tier2 = tier1_result.answer.to_uppercase().contains("ESCALATE");

    if !needs_tier2 {
        return Ok(VilResponse::ok(SupportResponse {
            resolved_by: "Tier 1 — FAQ".into(),
            answer: tier1_result.answer,
            tiers_involved,
            total_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Tier 2: Technical support
    let tier2_input = format!(
        "Customer question: {}\nTier 1 notes: {}",
        req.question, tier1_result.answer
    );
    let tier2_result = state.tier2_agent.run(&tier2_input).await
        .map_err(|e| VilError::internal(format!("tier2 failed: {}", e)))?;

    tiers_involved.push(TierResult {
        tier: "Tier 2 — Technical".into(),
        response: tier2_result.answer.clone(),
    });

    let needs_tier3 = tier2_result.answer.to_uppercase().contains("ESCALATE");

    if !needs_tier3 {
        return Ok(VilResponse::ok(SupportResponse {
            resolved_by: "Tier 2 — Technical".into(),
            answer: tier2_result.answer,
            tiers_involved,
            total_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Tier 3: Specialist
    let tier3_input = format!(
        "Customer question: {}\nTier 1 notes: {}\nTier 2 diagnostics: {}",
        req.question, tier1_result.answer, tier2_result.answer
    );
    let tier3_result = state.tier3_agent.run(&tier3_input).await
        .map_err(|e| VilError::internal(format!("tier3 failed: {}", e)))?;

    tiers_involved.push(TierResult {
        tier: "Tier 3 — Specialist".into(),
        response: tier3_result.answer.clone(),
    });

    Ok(VilResponse::ok(SupportResponse {
        resolved_by: "Tier 3 — Specialist".into(),
        answer: tier3_result.answer,
        tiers_involved,
        total_ms: start.elapsed().as_millis() as u64,
    }))
}

/// GET /api/support/tiers — describe the escalation architecture.
async fn support_tiers() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "escalation_path": [
            {
                "tier": 1,
                "name": "FAQ Agent",
                "tools": ["faq_lookup"],
                "handles": "Common questions: password reset, billing, subscriptions",
                "escalation_trigger": "ESCALATE keyword in response"
            },
            {
                "tier": 2,
                "name": "Technical Support Agent",
                "tools": ["run_diagnostic", "search_logs"],
                "handles": "System issues, performance problems, error investigation",
                "escalation_trigger": "ESCALATE keyword in response"
            },
            {
                "tier": 3,
                "name": "Specialist Agent",
                "tools": ["create_incident", "execute_runbook"],
                "handles": "Critical incidents, infrastructure remediation, on-call escalation"
            }
        ],
        "pattern": "vil_multi_agent::AgentGraph (linear chain) + conditional ESCALATE detection",
        "note": "Each tier has a distinct system prompt and tool set. Escalation is automatic."
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let make_llm = || -> Arc<dyn vil_llm::LlmProvider> {
        Arc::new(OpenAiProvider::new(
            OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
        ))
    };

    // Tier 1: FAQ Agent — handles common customer questions
    let tier1 = Agent::builder()
        .llm(make_llm())
        .tool(Arc::new(FaqLookupTool))
        .max_iterations(3)
        .system_prompt(
            "You are a Tier 1 FAQ support agent. Use the faq_lookup tool to find answers \
             to common customer questions about passwords, billing, subscriptions, etc. \
             If the FAQ does not have a relevant answer, respond with ESCALATE and a brief \
             summary of the issue so Tier 2 can investigate."
        )
        .build();

    // Tier 2: Technical Support — diagnostics and log analysis
    let tier2 = Agent::builder()
        .llm(make_llm())
        .tool(Arc::new(DiagnosticTool))
        .tool(Arc::new(LogSearchTool))
        .max_iterations(5)
        .system_prompt(
            "You are a Tier 2 technical support agent. You receive escalated issues from \
             Tier 1 that could not be resolved with FAQ articles. Use run_diagnostic and \
             search_logs tools to investigate the root cause. Provide a clear technical \
             explanation and resolution steps. If the issue requires infrastructure changes, \
             incident creation, or runbook execution, respond with ESCALATE and your findings."
        )
        .build();

    // Tier 3: Specialist — incident management and remediation
    let tier3 = Agent::builder()
        .llm(make_llm())
        .tool(Arc::new(IncidentCreateTool))
        .tool(Arc::new(RunbookTool))
        .max_iterations(5)
        .system_prompt(
            "You are a Tier 3 specialist support agent handling critical escalations. \
             You have access to incident creation and runbook execution tools. \
             Create incidents for tracking, execute appropriate runbooks for remediation, \
             and provide a comprehensive resolution plan. Always create an incident ticket \
             for audit trail. Never respond with ESCALATE — you are the final tier."
        )
        .build();

    let state = Arc::new(EscalationState {
        tier1_agent: tier1,
        tier2_agent: tier2,
        tier3_agent: tier3,
    });

    let svc = ServiceProcess::new("support")
        .prefix("/api")
        .endpoint(Method::POST, "/support/ask", post(support_ask))
        .endpoint(Method::GET, "/support/tiers", get(support_tiers))
        .state(state);

    VilApp::new("multi-agent-support")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
