// ╔════════════════════════════════════════════════════════════╗
// ║  302 — Legal Compliance Search Engine                     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ║  Domain:   Search across regulations, case law, and       ║
// ║            internal policies — merge and rank results      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p rag-plugin-usage-tech-docs
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "How does VIL routing work?"}' \
//     http://localhost:3111/api/multi-rag
//
// BUSINESS CONTEXT:
//   Legal compliance search engine for a regulated enterprise. Compliance
//   officers need to answer questions like "What are the data retention
//   requirements?" by searching across multiple knowledge bases:
//     KB 1 (Tech Docs) — internal architecture documentation, API specs
//     KB 2 (FAQ/Policy) — compliance FAQ, operational procedures
//   The fan-in pattern searches both independently, cross-ranks by relevance,
//   and merges the top results into a unified context for the LLM. This
//   ensures answers cite BOTH regulatory requirements AND internal policies.
//
// HOW THIS DIFFERS FROM 301:
//   301 = single document collection + cosine similarity
//   302 = TWO separate knowledge bases (tech docs + FAQ),
//         search both independently, cross-rank results by
//         relevance score, combine top hits into unified context

use vil_rag::semantic::{RagFault, RagIndexState, RagIngestEvent, RagQueryEvent};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct MultiSourceState {
    pub total_queries: u64,
    pub tech_hits: u64,
    pub faq_hits: u64,
    pub cross_source_queries: u64,
}

#[derive(Clone, Debug)]
pub struct MultiSourceSearchEvent {
    pub query: String,
    pub tech_results: u32,
    pub faq_results: u32,
    pub merged_top_k: u32,
}

#[vil_fault]
pub enum MultiSourceFault {
    BothSourcesEmpty,
    RankingFailed,
    SourceTimeout,
}

// ── Knowledge Base 1: Technical Documentation ───────────────────────
// In a legal compliance context, this would be the regulatory corpus
// (e.g., GDPR articles, OJK regulations, SOX requirements).
// Each doc has a unique ID for citation tracking in LLM responses.

struct TechDoc {
    id: &'static str,
    content: &'static str,
    keywords: &'static [&'static str],
}

const TECH_DOCS: &[TechDoc] = &[
    TechDoc {
        id: "T1",
        content: "VIL Tri-Lane Model: Data Lane carries payload messages, Control \
                  Lane carries fault signals and lifecycle commands, and Trigger Lane \
                  carries activation events. All three lanes operate over shared memory.",
        keywords: &["tri-lane", "routing", "data", "control", "trigger", "lane"],
    },
    TechDoc {
        id: "T2",
        content: "VIL SHM (Shared Memory): VastarRuntimeWorld manages memory-mapped \
                  regions. Processes exchange messages via LoanWrite (zero-copy) or Copy \
                  (small control). No serialization overhead for in-process communication.",
        keywords: &["shm", "shared", "memory", "loanwrite", "copy", "zero-copy"],
    },
    TechDoc {
        id: "T3",
        content: "vil_workflow! macro: Layer E API for multi-stage pipelines. Accepts a \
                  name, process instances, and route specifications. Routes wire ports \
                  between processes with transfer mode: LoanWrite for data, Copy for control.",
        keywords: &["workflow", "pipeline", "macro", "route", "builder"],
    },
    TechDoc {
        id: "T4",
        content: "VilApp: Layer F high-level server framework. Wraps axum with \
                  ServiceProcess for semantic endpoint registration. Supports emits/faults/manages \
                  for tri-lane type safety. Best for HTTP microservices.",
        keywords: &["vilapp", "server", "http", "axum", "service", "endpoint"],
    },
];

// ── Knowledge Base 2: FAQ / Support ─────────────────────────────────
// In a legal compliance context, this would be the internal policy KB
// (e.g., data handling procedures, audit checklists, exception processes).

struct FaqDoc {
    id: &'static str,
    question: &'static str,
    answer: &'static str,
    keywords: &'static [&'static str],
}

const FAQ_DOCS: &[FaqDoc] = &[
    FaqDoc {
        id: "F1",
        question: "How do I create a new pipeline?",
        answer: "Use vil_workflow! macro. Define instances (sink + source), then wire \
                 routes between their ports. Run with HttpSink/HttpSource workers.",
        keywords: &["create", "pipeline", "new", "how", "workflow"],
    },
    FaqDoc {
        id: "F2",
        question: "What is the difference between LoanWrite and Copy?",
        answer: "LoanWrite is zero-copy (lends a memory region). Copy duplicates the \
                 data. Use LoanWrite for large payloads (data lane), Copy for small \
                 control messages (control lane).",
        keywords: &["loanwrite", "copy", "difference", "transfer"],
    },
    FaqDoc {
        id: "F3",
        question: "How do I add authentication?",
        answer: "Use bearer_token() for OpenAI/Anthropic, api_key_param() for Gemini, \
                 or anthropic_key() for Anthropic x-api-key header. Set via env vars.",
        keywords: &["auth", "authentication", "api", "key", "bearer", "token"],
    },
];

// ── Relevance Scoring ───────────────────────────────────────────────

/// Score a document against query using keyword overlap.
/// Business: in production, this would be replaced with vector cosine
/// similarity (embedding-based). Keyword scoring is used here as a
/// zero-dependency approximation for the demo.
fn keyword_score(query: &str, keywords: &[&str]) -> f64 {
    let q = query.to_lowercase();
    let q_words: Vec<&str> = q.split_whitespace().collect();
    let mut matches = 0;
    for kw in keywords {
        if q.contains(kw) || q_words.iter().any(|w| w.contains(kw)) {
            matches += 1;
        }
    }
    if keywords.is_empty() {
        0.0
    } else {
        matches as f64 / keywords.len() as f64
    }
}

#[derive(Clone, Debug, Serialize)]
struct RankedResult {
    source: String, // "tech" or "faq"
    doc_id: String,
    score: f64,
    content: String,
}

// ── Request / Response ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct MultiRagRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct SourceResult {
    source: String,
    doc_id: String,
    score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct MultiRagResponse {
    content: String,
    sources_searched: u32,
    results_merged: Vec<SourceResult>,
}

// ── Handler ──────────────────────────────────────────────────────────

async fn multi_rag_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<MultiRagResponse>> {
    let req: MultiRagRequest = body.json().expect("invalid JSON body");
    // Step 1: Search BOTH knowledge bases independently.
    // Business: parallel search is critical — compliance queries have SLA < 3s.
    // In production, each KB would be a separate vector DB with dedicated indexes.
    let mut all_results: Vec<RankedResult> = Vec::new();

    // Search tech docs
    for doc in TECH_DOCS {
        let score = keyword_score(&req.prompt, doc.keywords);
        if score > 0.0 {
            all_results.push(RankedResult {
                source: "tech".into(),
                doc_id: doc.id.into(),
                score,
                content: doc.content.to_string(),
            });
        }
    }

    // Search FAQ docs
    for doc in FAQ_DOCS {
        let score = keyword_score(&req.prompt, doc.keywords);
        if score > 0.0 {
            all_results.push(RankedResult {
                source: "faq".into(),
                doc_id: doc.id.into(),
                score,
                content: format!("Q: {} A: {}", doc.question, doc.answer),
            });
        }
    }

    // Step 2: Cross-rank by score (descending) and take top 3.
    // Cross-ranking merges results from different sources into a single
    // relevance-ordered list — the key innovation of multi-source RAG.
    all_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_results: Vec<&RankedResult> = all_results.iter().take(3).collect();

    // If no results, include fallback context
    let context = if top_results.is_empty() {
        "No relevant documents found in either knowledge base.".to_string()
    } else {
        top_results
            .iter()
            .map(|r| {
                format!(
                    "[{}:{}] (score: {:.2}) {}",
                    r.source, r.doc_id, r.score, r.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    // Step 3: Build LLM prompt with merged context
    let system_prompt = format!(
        "You are a VIL assistant with access to TWO knowledge bases:\n\
         - tech: Technical documentation\n\
         - faq: Frequently asked questions\n\n\
         Answer using ONLY the context below. Cite sources as [source:DocID].\n\n\
         Context:\n{}",
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

    let results_merged: Vec<SourceResult> = top_results
        .iter()
        .map(|r| SourceResult {
            source: r.source.clone(),
            doc_id: r.doc_id.clone(),
            score: (r.score * 100.0).round() / 100.0,
        })
        .collect();

    // Semantic audit
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: results_merged.len() as u32,
        answer_length: content.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(MultiRagResponse {
        content,
        sources_searched: 2,
        results_merged,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let _ = std::any::type_name::<RagQueryEvent>();
    let _ = std::any::type_name::<RagIngestEvent>();
    let _ = std::any::type_name::<RagFault>();
    let _ = std::any::type_name::<RagIndexState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  302 — RAG Multi-Source Fan-In (VilApp)                    ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Two KB sources + cross-ranking + merged context   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Knowledge bases:");
    println!(
        "    tech_docs: {} documents (VIL architecture)",
        TECH_DOCS.len()
    );
    println!("    faq_docs : {} documents (support Q&A)", FAQ_DOCS.len());
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
    println!("  Listening on http://localhost:3111/api/multi-rag");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("rag-multi-source")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/multi-rag", post(multi_rag_handler));

    VilApp::new("rag-multi-source-fanin")
        .port(3111)
        .service(svc)
        .run()
        .await;
}
