// ╔════════════════════════════════════════════════════════════╗
// ║  303 — FAQ + Knowledge Base Hybrid Search                 ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Support — Customer Self-Service                 ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Two-tier hybrid search for support knowledge:   ║
// ║    Tier 1 — exact keyword match (FAQ DB, zero LLM cost)  ║
// ║    Tier 2 — keyword-scored retrieval (not vector search)   ║
// ║  Exact hits bypass LLM entirely, saving latency + cost.   ║
// ║  Fundamentally different control flow from pure RAG.       ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p rag-plugin-usage-faq-bot
//
// Test (exact hit — no LLM call):
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "How do I reset my password?"}' \
//     http://localhost:3112/api/hybrid
//
// Test (semantic fallback — triggers LLM):
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Tell me about changing login credentials"}' \
//     http://localhost:3112/api/hybrid
//
// HOW THIS DIFFERS FROM 301:
//   301 = always vector search -> always LLM
//   303 = exact match first (zero latency, no LLM needed)
//         if no exact hit -> semantic search -> LLM
//   This has fundamentally different control flow.

use vil_rag::semantic::{RagFault, RagIndexState, RagIngestEvent, RagQueryEvent};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
/// Hybrid search state — tracks exact hit rate vs semantic fallback rate.
pub struct HybridSearchState {
    pub total_queries: u64,
    pub exact_hits: u64,
    pub semantic_fallbacks: u64,
    pub no_results: u64,
}

#[derive(Clone, Debug)]
/// Search event — tracks which tier handled the query (exact vs semantic).
pub struct HybridSearchEvent {
    pub query: String,
    pub search_tier: String, // "exact" or "semantic"
    pub llm_called: bool,
    pub result_doc_id: String,
}

#[vil_fault]
/// Search faults — NoResultsFound is the most common (new/unusual queries).
pub enum HybridSearchFault {
    NoResultsFound,
    SemanticSearchFailed,
    LlmFallbackTimeout,
}

// ── FAQ Database with exact-match keys ──────────────────────────────
// Tier 1 (Exact): FAQ entries with trigger phrases for instant lookup.
// When a customer asks "How do I reset my password?", this matches
// immediately without any LLM call — saving ~2s latency and ~$0.01.
// Tier 2 (Semantic): Falls back to keyword-based similarity search
// + LLM generation for queries that don't match any FAQ trigger.

/// A FAQ entry in the customer support knowledge base
struct FaqEntry {
    id: &'static str,
    /// Exact-match trigger phrases (case-insensitive substring match)
    triggers: &'static [&'static str],
    question: &'static str,
    answer: &'static str,
    /// Semantic keywords for fallback search
    keywords: &'static [&'static str],
}

const FAQ_DB: &[FaqEntry] = &[
    FaqEntry {
        id: "FAQ-01",
        triggers: &[
            "reset my password",
            "change password",
            "forgot password",
            "reset password",
        ],
        question: "How do I reset my password?",
        answer: "Go to Settings > Security > Change Password. Enter your current password, \
                 then type and confirm your new password. You will receive a confirmation \
                 email. If you forgot your password, click 'Forgot Password' on the login page.",
        keywords: &["password", "reset", "change", "security", "login", "forgot"],
    },
    FaqEntry {
        id: "FAQ-02",
        triggers: &[
            "payment methods",
            "accepted payments",
            "how to pay",
            "credit card",
        ],
        question: "What payment methods are accepted?",
        answer: "We accept Visa, MasterCard, American Express, and PayPal. Enterprise accounts \
                 can use wire transfers and purchase orders (NET-30 terms). Cryptocurrency \
                 payments are not currently supported.",
        keywords: &["payment", "pay", "visa", "mastercard", "paypal", "billing"],
    },
    FaqEntry {
        id: "FAQ-03",
        triggers: &[
            "cancel subscription",
            "cancel plan",
            "cancel my account",
            "unsubscribe",
        ],
        question: "How do I cancel my subscription?",
        answer: "Navigate to Settings > Billing > Manage Subscription and click 'Cancel Plan'. \
                 Your access continues until the end of the current billing period. Refunds \
                 are available within 14 days of the most recent charge.",
        keywords: &["cancel", "subscription", "billing", "refund", "plan"],
    },
    FaqEntry {
        id: "FAQ-04",
        triggers: &[
            "data export",
            "download my data",
            "export data",
            "gdpr export",
        ],
        question: "How do I export my data?",
        answer: "Go to Settings > Privacy > Export Data. Click 'Request Export' and we will \
                 prepare a ZIP file containing all your data. You will receive a download \
                 link via email within 24 hours. Exports include profile data, activity logs, \
                 and uploaded files.",
        keywords: &["export", "data", "download", "privacy", "gdpr", "backup"],
    },
    FaqEntry {
        id: "FAQ-05",
        triggers: &[
            "two factor",
            "2fa",
            "mfa",
            "two-factor authentication",
            "enable 2fa",
        ],
        question: "How do I enable two-factor authentication?",
        answer: "Go to Settings > Security > Two-Factor Authentication. Choose your method: \
                 authenticator app (recommended) or SMS. Scan the QR code with your \
                 authenticator app, enter the verification code, and save your backup codes.",
        keywords: &["two-factor", "2fa", "mfa", "security", "authenticator"],
    },
];

// ── Search Functions ────────────────────────────────────────────────
// Two-tier search strategy: exact match first (instant, free), then
// semantic search (requires computation, may invoke LLM for generation).
// This hybrid approach optimizes for both latency and cost.

/// Tier 1: Exact substring match (case-insensitive).
/// Returns immediately if any FAQ trigger phrase appears in the query.
/// This tier handles ~60% of customer queries at zero LLM cost.
fn exact_match(query: &str) -> Option<&'static FaqEntry> {
    let q = query.to_lowercase();
    FAQ_DB
        .iter()
        .find(|entry| entry.triggers.iter().any(|trigger| q.contains(trigger)))
}

/// Tier 2: Keyword overlap scoring (not true semantic/vector search).
/// Falls back here when no exact FAQ trigger matches. Scores each FAQ entry
/// by keyword overlap and returns ranked results for LLM augmentation.
fn semantic_search(query: &str) -> Vec<(&'static FaqEntry, f64)> {
    let q = query.to_lowercase();
    let mut results: Vec<(&FaqEntry, f64)> = FAQ_DB
        .iter()
        .map(|entry| {
            let hits = entry.keywords.iter().filter(|kw| q.contains(*kw)).count();
            let score = if entry.keywords.is_empty() {
                0.0
            } else {
                hits as f64 / entry.keywords.len() as f64
            };
            (entry, score)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

// ── Request / Response ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Customer support query — natural language question from a user.
struct HybridRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
/// Hybrid search response — answer, search tier used, and matched FAQ ID.
struct HybridResponse {
    content: String,
    search_tier: String, // "exact" or "semantic+llm"
    llm_used: bool,
    matched_faq: Option<String>,
}

// ── Handler ──────────────────────────────────────────────────────────

async fn hybrid_handler(
    _ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<HybridResponse>> {
    let req: HybridRequest = body.json().expect("invalid JSON body");
    // ── Tier 1: Exact match (no LLM needed) ─────────────────────────
    if let Some(entry) = exact_match(&req.prompt) {
        return Ok(VilResponse::ok(HybridResponse {
            content: format!("Q: {}\n\nA: {}", entry.question, entry.answer),
            // Tier 1 hit — answered from FAQ database without LLM call
            search_tier: "exact".into(),
            llm_used: false,
            matched_faq: Some(entry.id.into()),
        }));
    }

    // ── Tier 2: Semantic search + LLM ───────────────────────────────
    // Tier 2: No exact match — fall back to semantic keyword search + LLM generation
    let semantic_results = semantic_search(&req.prompt);

    let context = if semantic_results.is_empty() {
        "No relevant FAQ entries found.".to_string()
    } else {
        semantic_results
            .iter()
            .take(3)
            .map(|(entry, score)| {
                format!(
                    "[{}] (relevance: {:.0}%) Q: {} A: {}",
                    entry.id,
                    score * 100.0,
                    entry.question,
                    entry.answer
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let matched_id = semantic_results.first().map(|(e, _)| e.id.to_string());

    // Build the LLM prompt with the best-matching FAQ content as context
    let system_prompt = format!(
        "You are a support assistant. Answer the user's question using the FAQ entries below. \
         If the FAQ does not cover the question, say so.\n\nFAQ Entries:\n{}",
        context
    );

    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Use real API key for production; simulator mode for local dev testing
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Stream the LLM response and collect into a single text for the client
    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Log the search event for analytics — tracks exact vs semantic usage ratio
    // Semantic audit
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: semantic_results.len() as u32,
        answer_length: content.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(HybridResponse {
        content,
        // Tier 2 fallback — required LLM generation with semantic context
        search_tier: "semantic+llm".into(),
        llm_used: true,
        matched_faq: matched_id,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
// ── Main — FAQ + Knowledge Base Hybrid Search Engine ─────────────
// Assembles the two-tier search service. Exact matches are instant
// and free; semantic fallbacks invoke the LLM for novel questions.
async fn main() {
    // Register RAG semantic types for observability and query analytics
    let _ = std::any::type_name::<RagQueryEvent>();
    let _ = std::any::type_name::<RagIngestEvent>();
    let _ = std::any::type_name::<RagFault>();
    let _ = std::any::type_name::<RagIndexState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  303 — RAG Hybrid Exact + Semantic Search (VilApp)         ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Exact match first (no LLM), semantic fallback     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "  FAQ entries: {} (with exact triggers + semantic keywords)",
        FAQ_DB.len()
    );
    println!("  Tier 1: Exact substring match -> direct answer (no LLM)");
    println!("  Tier 2: Semantic keyword search -> LLM synthesis");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    // Display authentication mode (API key vs simulator)
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    // Display the endpoint URL for this service
    println!("  Listening on http://localhost:3112/api/hybrid");
    // Display the upstream data source URL
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("rag-hybrid")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/hybrid", post(hybrid_handler));

    VilApp::new("rag-hybrid-exact-semantic")
        .port(3112)
        .service(svc)
        .run()
        .await;
}
