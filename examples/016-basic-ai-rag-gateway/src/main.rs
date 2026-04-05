// ╔════════════════════════════════════════════════════════════╗
// ║  016 — Enterprise Knowledge Search (RAG Pipeline)         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Enterprise — Internal Knowledge Management      ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    GenericToken                                     ║
// ║  Features: vil_workflow!, .transform() with real retrieval ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: RAG pipeline — employee posts question,         ║
// ║  transform() searches knowledge base, injects relevant     ║
// ║  documents into the LLM prompt, streams grounded answer.   ║
// ║  Real keyword retrieval, not just a static system prompt.  ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-basic-ai-rag-gateway
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "How do I reset my VPN password?"}' \
//     http://localhost:3084/rag

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ───────────────────────────────────────────────────────

#[vil_state]
pub struct RagState {
    pub request_id: u64,
    pub context_docs_used: u8,
    pub tokens_received: u32,
}

#[vil_event]
pub struct RagResponse {
    pub request_id: u64,
    pub total_tokens: u32,
    pub docs_cited: u8,
    pub duration_ns: u64,
}

#[vil_fault]
pub enum RagFault {
    UpstreamTimeout,
    ContextRetrievalFailed,
    SseParseError,
}

// ── Knowledge Base (real documents) ──────────────────────────────────────

struct KbArticle {
    id: &'static str,
    title: &'static str,
    content: &'static str,
    keywords: &'static [&'static str],
}

const KNOWLEDGE_BASE: &[KbArticle] = &[
    KbArticle {
        id: "KB-001", title: "Password Reset Procedure",
        content: "To reset your password: 1) Go to https://password.company.com 2) Click 'Forgot Password' 3) Enter your corporate email 4) Check inbox (and spam) for reset link 5) Create new password (min 12 chars, 1 uppercase, 1 number, 1 special). If locked after 5 failed attempts, contact IT helpdesk.",
        keywords: &["password", "reset", "forgot", "locked", "login"],
    },
    KbArticle {
        id: "KB-002", title: "VPN Setup and Troubleshooting",
        content: "VPN Setup: Install GlobalProtect from Software Center. Server: vpn.company.com. Troubleshooting: 1) Restart GlobalProtect client 2) Check internet connectivity 3) Try alternate DNS (8.8.8.8) 4) Clear VPN cache: delete ~/.gpcache 5) Reinstall client if persistent. For MFA issues with VPN, re-enroll at https://mfa.company.com.",
        keywords: &["vpn", "globalprotect", "connect", "tunnel", "remote"],
    },
    KbArticle {
        id: "KB-003", title: "Email and Calendar Configuration",
        content: "Outlook Desktop: File > Account Settings > Repair. Outlook Mobile: Remove account, re-add with corporate email. Calendar sync: check delegate permissions in OWA (outlook.office.com). Shared mailbox: File > Open > Other User's Mailbox. Max attachment: 25MB (use SharePoint for larger files).",
        keywords: &["email", "outlook", "calendar", "sync", "mailbox"],
    },
    KbArticle {
        id: "KB-004", title: "Software Installation Policy",
        content: "Approved software: use Software Center (self-service, no admin needed). Non-standard software: submit request at https://it.company.com/software-request with business justification. Typical approval: 2-3 business days. Prohibited: cryptocurrency miners, torrent clients, unlicensed software. Admin rights: not granted by default, request via manager approval.",
        keywords: &["software", "install", "application", "download", "program"],
    },
    KbArticle {
        id: "KB-005", title: "Multi-Factor Authentication (MFA)",
        content: "Setup: 1) Download Microsoft Authenticator 2) Go to https://mfa.company.com 3) Scan QR code 4) Verify with 6-digit code. Backup: save recovery codes in secure location. Lost phone: contact IT for temporary bypass (valid 24h). Hardware token available for users without smartphones.",
        keywords: &["mfa", "authenticator", "two-factor", "2fa", "token"],
    },
    KbArticle {
        id: "KB-006", title: "WiFi Network Access",
        content: "Corporate WiFi: SSID 'Corp-Secure' (802.1x, use AD credentials). Guest WiFi: 'Corp-Guest' (captive portal, 24h access). Eduroam available for visiting academics. Troubleshooting: forget network, reconnect. If certificate error, sync time and re-join domain.",
        keywords: &["wifi", "wireless", "network", "ssid", "internet"],
    },
    KbArticle {
        id: "KB-007", title: "Printer Setup and Troubleshooting",
        content: "Add printer: Settings > Printers > Add. Network printers auto-discovered via AD. Manual: IP range 10.0.1.x. If offline: 1) Check physical connection 2) Restart print spooler (services.msc) 3) Clear print queue 4) Reinstall driver from \\\\printserver\\drivers.",
        keywords: &["printer", "print", "scanning", "paper", "jam"],
    },
    KbArticle {
        id: "KB-008", title: "File Sharing and Collaboration",
        content: "SharePoint: company.sharepoint.com (department sites). OneDrive: personal work files (1TB). Shared drives: \\\\fileserver\\department (legacy, migrating to SharePoint). External sharing: SharePoint external links (requires manager approval). Max file size: 250GB on SharePoint, 25MB email attachment.",
        keywords: &["file", "share", "sharepoint", "onedrive", "drive", "collaboration"],
    },
    KbArticle {
        id: "KB-009", title: "New Employee IT Onboarding",
        content: "Day 1: AD account + email created by HR ticket. Day 1: collect laptop from IT (building B, room 102). Day 2: VPN + MFA setup with IT buddy. Week 1: department-specific tools provisioned. Checklist: https://it.company.com/onboarding. Issues: helpdesk@company.com or ext. 4357.",
        keywords: &["new", "employee", "onboarding", "first", "day", "start"],
    },
    KbArticle {
        id: "KB-010", title: "Data Backup and Recovery",
        content: "OneDrive auto-syncs desktop/documents (version history 30 days). SharePoint: site-level backup daily (90-day retention). Local backup: not supported — use OneDrive. Recovery request: submit ticket with file path and approximate date. SLA: P3 (24h) for recovery requests.",
        keywords: &["backup", "recover", "restore", "lost", "deleted", "file"],
    },
];

/// Search knowledge base — keyword scoring, return top 3 relevant articles.
fn search_kb(query: &str) -> Vec<(f64, &'static KbArticle)> {
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();

    let mut scored: Vec<(f64, &KbArticle)> = KNOWLEDGE_BASE
        .iter()
        .map(|article| {
            let keyword_hits = article.keywords.iter()
                .filter(|kw| words.iter().any(|w| w.contains(*kw) || kw.contains(w)))
                .count();
            let title_hits = words.iter()
                .filter(|w| article.title.to_lowercase().contains(*w))
                .count();
            let content_hits = words.iter()
                .filter(|w| article.content.to_lowercase().contains(*w))
                .count();
            let score = keyword_hits as f64 * 3.0 + title_hits as f64 * 2.0 + content_hits as f64;
            (score, article)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    scored.into_iter().take(3).collect()
}

// ── Pipeline Configuration ───────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3084;
const WEBHOOK_PATH: &str = "/rag";
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const SSE_JSON_TAP: &str = "choices[0].delta.content";

fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("RagWebhook")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_source() -> HttpSourceBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = HttpSourceBuilder::new("RagSseInference")
        .url(SSE_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::OpenAi)
        .json_tap(SSE_JSON_TAP)
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        // Transform: intercept incoming request, search KB, inject context into prompt
        .transform(|body: &[u8]| {
            // Parse user query from incoming request body
            let req: serde_json::Value = serde_json::from_slice(body).ok()?;
            let user_prompt = req["prompt"].as_str().unwrap_or("help");

            // REAL retrieval: search knowledge base
            let results = search_kb(user_prompt);
            let context = if results.is_empty() {
                "No relevant documents found.".to_string()
            } else {
                results.iter()
                    .map(|(score, article)| {
                        format!("[{}] {} (relevance: {:.1})\n{}", article.id, article.title, score, article.content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };

            // Build RAG prompt with retrieved context
            let rag_body = serde_json::json!({
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": format!(
                            "You are an IT helpdesk assistant. Answer based on the following knowledge base documents. \
                             Always cite the document ID (e.g. [KB-002]) when referencing information.\n\n\
                             --- RETRIEVED DOCUMENTS ---\n{}\n--- END DOCUMENTS ---",
                            context
                        )
                    },
                    { "role": "user", "content": user_prompt }
                ],
                "stream": true
            });

            Some(serde_json::to_vec(&rag_body).ok()?)
        });

    if !api_key.is_empty() {
        builder = builder.bearer_token(&api_key);
    }

    builder
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() {
    let world = Arc::new(
        VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM Runtime"),
    );

    let sink_builder = configure_sink();
    let source_builder = configure_source();

    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "RagPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  016 — RAG Pipeline with Real Knowledge Base Retrieval      ║");
    println!("║  {} KB articles indexed, keyword-scored retrieval    ║", KNOWLEDGE_BASE.len());
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Listening: http://localhost:{}{}", WEBHOOK_PORT, WEBHOOK_PATH);
    println!("  Upstream:  {}", SSE_URL);
    println!("  KB Size:   {} articles", KNOWLEDGE_BASE.len());
    println!();

    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink panicked");
    t2.join().expect("Source panicked");
}
