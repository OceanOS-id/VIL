// 026 — IT Helpdesk Agent (VWFD)
// Business logic matches standard:
//   - system_status tool: 8 services with status + uptime
//   - knowledge_base tool: 10 KB articles with keyword scoring (top 3)
//   - Response: answer + tools_used + iterations
// Note: standard uses multi-turn LLM (SseCollect). VWFD uses rule-based
// tool dispatch (same tools, same data, no LLM).
use serde_json::{json, Value};

const SERVICES: &[(&str, &str, &str)] = &[
    ("vpn", "online", "99.8%"),
    ("email", "online", "99.9%"),
    ("active_directory", "online", "100%"),
    ("dns", "online", "99.99%"),
    ("file_server", "online", "99.5%"),
    ("print_server", "degraded", "95.2%"),
    ("erp", "online", "99.7%"),
    ("wifi", "online", "99.3%"),
];

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

fn tool_system_status(service: &str) -> String {
    if service == "all" || service.is_empty() {
        SERVICES.iter()
            .map(|(n, s, u)| format!("{}: {} (uptime {})", n, s, u))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        SERVICES.iter()
            .find(|(n, _, _)| *n == service)
            .map(|(n, s, u)| format!("{}: {} (uptime {})", n, s, u))
            .unwrap_or_else(|| format!("{}: unknown service", service))
    }
}

fn tool_knowledge_base(query: &str) -> String {
    let q = query.to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

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

    if top3.is_empty() {
        "No matching KB articles found.".into()
    } else {
        top3.iter()
            .map(|(score, id, title, content)| format!("[{}] {} (score={})\n{}", id, title, score, content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn react_agent(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let query = body["query"].as_str()
        .or_else(|| body["prompt"].as_str())
        .unwrap_or("");

    let mut tools_used = Vec::new();
    let mut observations = Vec::new();

    // Step 1: Always check system status
    let status = tool_system_status("all");
    tools_used.push("system_status");
    observations.push(format!("System status:\n{}", status));

    // Step 2: Search knowledge base
    let kb_result = tool_knowledge_base(query);
    tools_used.push("knowledge_base");
    observations.push(format!("KB search:\n{}", kb_result));

    // Step 3: Compose answer from observations
    let answer = format!(
        "Based on analysis of '{}': {}\n\nRelevant documentation:\n{}",
        query,
        if status.contains("degraded") { "Some services are degraded." } else { "All systems operational." },
        kb_result
    );

    Ok(json!({
        "answer": answer,
        "tools_used": tools_used,
        "iterations": 2
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/026-basic-ai-agent/vwfd/workflows", 8080)
        .native("react_agent_loop", react_agent)
        .run().await;
}
