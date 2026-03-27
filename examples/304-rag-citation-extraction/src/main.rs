// ╔════════════════════════════════════════════════════════════╗
// ║  304 — Academic Research Assistant                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Research — Academic & Legal Document Search      ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Generates answers with precise source           ║
// ║  citations in [DocN] format. Critical for research,        ║
// ║  legal compliance, and audit scenarios where every claim  ║
// ║  must be traceable to a specific source document.          ║
// ║  Prevents LLM hallucination through grounded retrieval.   ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p rag-plugin-usage-legal-search
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What are the termination conditions?"}' \
//     http://localhost:3113/api/cited-rag
//
// HOW THIS DIFFERS FROM 301:
//   301 = RAG -> LLM -> return raw text
//   304 = RAG -> LLM -> POST-PROCESS response to extract [Doc1], [Doc2]
//         references -> build structured citations array with title, snippet,
//         relevance -> return { answer, citations[] }

use vil_server::prelude::*;
use vil_rag::semantic::{RagQueryEvent, RagIngestEvent, RagFault, RagIndexState};

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
/// Citation search state — tracks query volume and citation extraction accuracy.
pub struct CitationState {
    pub total_queries: u64,
    pub total_citations_extracted: u64,
    pub avg_citations_per_query: f64,
}

#[derive(Clone, Debug)]
pub struct CitationExtractedEvent {
    pub query: String,
    pub answer_length: u32,
    pub citations_found: u32,
    pub unique_docs_cited: u32,
}

#[vil_fault]
/// Citation faults — InvalidCitationFormat = LLM produced non-[DocN] reference.
pub enum CitationFault {
    NoCitationsFound,
    InvalidCitationFormat,
    DocumentNotInIndex,
    LlmUpstreamError,
}

// ── Document Index (legal contract clauses) ─────────────────────────
// Indexed legal documents (contracts, policies, regulations). In production,
// these would be stored in a vector DB with real embeddings. The citation
// system ensures every claim in the LLM response is traceable to a specific
// section — critical for legal compliance and audit requirements.

/// A section from a legal document in the research corpus
struct LegalDoc {
    id: &'static str,
    section: &'static str,
    title: &'static str,
    content: &'static str,
}

const LEGAL_DOCS: &[LegalDoc] = &[
    LegalDoc {
        id: "Doc1",
        section: "5.1",
        title: "Termination for Convenience",
        content: "Either party may terminate this Agreement upon thirty (30) days prior \
                  written notice. Upon termination, the Licensee shall cease all use of \
                  the Software and destroy all copies within ten (10) business days.",
    },
    LegalDoc {
        id: "Doc2",
        section: "5.2",
        title: "Termination for Cause",
        content: "Either party may terminate immediately upon written notice if the other \
                  party materially breaches this Agreement and fails to cure such breach \
                  within fifteen (15) days after receiving written notice of the breach.",
    },
    LegalDoc {
        id: "Doc3",
        section: "7.3",
        title: "Limitation of Liability",
        content: "In no event shall either party be liable for indirect, incidental, \
                  special, consequential, or punitive damages. Total cumulative liability \
                  shall not exceed the fees paid in the preceding twelve (12) months.",
    },
    LegalDoc {
        id: "Doc4",
        section: "9.2",
        title: "Confidentiality Obligations",
        content: "Each party agrees to hold in confidence all Confidential Information. \
                  These obligations survive for five (5) years after termination or \
                  expiration of this Agreement.",
    },
    LegalDoc {
        id: "Doc5",
        section: "11.1",
        title: "Governing Law",
        content: "This Agreement shall be governed by the laws of the State of Delaware, \
                  without regard to conflict of laws principles. Any disputes shall be \
                  resolved in the state or federal courts located in Wilmington, Delaware.",
    },
];

// ── Citation Extraction (post-processing) ───────────────────────────
// After the LLM generates a response, this extracts [DocN] references
// and builds structured citation objects with section, title, and snippet.
// Researchers and legal teams need these for verification and audit trails.

/// Structured citation linking an LLM claim to a specific source document
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Citation {
    doc_id: String,
    section: String,
    title: String,
    snippet: String,
    mention_count: u32,
}

/// Extract [DocN] references from LLM output and build structured citations
fn extract_citations(llm_text: &str) -> Vec<Citation> {
    let mut citations = Vec::new();
    let mut seen = std::collections::HashMap::new();

    // Parse the LLM output for [Doc1], [Doc2], etc. citation markers
    // Find all [DocN] patterns
    let mut pos = 0;
    while pos < llm_text.len() {
        if let Some(start) = llm_text[pos..].find("[Doc") {
            let abs_start = pos + start;
            if let Some(end) = llm_text[abs_start..].find(']') {
                let ref_text = &llm_text[abs_start + 1..abs_start + end]; // e.g. "Doc1"
                let count = seen.entry(ref_text.to_string()).or_insert(0u32);
                *count += 1;
                pos = abs_start + end + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Map each [DocN] reference to its source section, title, and snippet for verification
    // Build citation objects from extracted references
    for (ref_id, count) in &seen {
        if let Some(doc) = LEGAL_DOCS.iter().find(|d| d.id == ref_id.as_str()) {
            // Extract a short snippet (first 120 chars)
            let snippet = if doc.content.len() > 120 {
                format!("{}...", &doc.content[..120])
            } else {
                doc.content.to_string()
            };

            citations.push(Citation {
                doc_id: doc.id.to_string(),
                section: format!("Section {}", doc.section),
                title: doc.title.to_string(),
                snippet,
                mention_count: *count,
            });
        }
    }

    // Sort by mention count (most cited first)
    // Most-cited documents first — shows which sources the LLM relied on most
    citations.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
    citations
}

// ── Request / Response ───────────────────────────────────────────────
// The response includes the generated answer, structured citations with
// section references, and similarity scores. This enables researchers to
// verify every claim in the answer against the original source document.

/// Research query — natural language question about legal/academic content
#[derive(Debug, Deserialize)]
struct CitedRagRequest {
    prompt: String,   // e.g., "What are the termination conditions in this contract?"
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct CitedRagResponse {
    answer: String,
    citations: Vec<Citation>,
    total_docs_in_context: u32,
    citation_count: u32,
}

// ── Handler ──────────────────────────────────────────────────────────
// The citation-aware RAG handler follows a 5-step research workflow:
//   1. Build context string from all indexed legal document sections
//   2. Construct the system prompt requiring [DocN] citation format
//   3. Send research question + context to LLM via SSE streaming
//   4. Post-process the LLM output to extract structured citations
//   5. Return answer + citations + provenance metadata
// This ensures every claim is traceable to a specific contract clause.

/// POST /api/legal — research query with citation extraction from legal corpus
async fn cited_rag_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<CitedRagResponse>> {
    let req: CitedRagRequest = body.json().expect("invalid JSON body");
    // Step 1: Build context from all indexed legal document sections
    let context: String = LEGAL_DOCS.iter()
        .map(|d| format!("[{}] Section {} — {}: {}", d.id, d.section, d.title, d.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    // Step 3: Build the research prompt — instruct LLM to cite sources as [DocN]
    let system_prompt = format!(
        "You are a legal document assistant. Answer using ONLY the clauses below. \
         You MUST cite every source used as [DocN] inline in your answer. \
         For example: 'According to [Doc1], termination requires...' \
         Cite every relevant document.\n\nContract Clauses:\n{}",
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
    // Configure SSE collector to stream from LLM and extract generated text
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    // Add Bearer token authentication if API key is configured
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let answer = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Step 2: POST-PROCESS — extract citations from LLM output
    // Step 4: Post-process LLM output to extract structured [DocN] citations
    let citations = extract_citations(&answer);
    let citation_count = citations.len() as u32;

    // Semantic audit
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: LEGAL_DOCS.len() as u32,
        answer_length: answer.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(CitedRagResponse {
        answer,
        citations,
        total_docs_in_context: LEGAL_DOCS.len() as u32,
        citation_count,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
// ── Main — Academic Research Assistant service ─────────────────────
// Assembles the citation-aware RAG service. The system prompt instructs
// the LLM to always cite sources as [DocN] for traceability.
async fn main() {
    // Register RAG semantic types for citation quality monitoring
    let _ = std::any::type_name::<RagQueryEvent>();
    let _ = std::any::type_name::<RagIngestEvent>();
    let _ = std::any::type_name::<RagFault>();
    let _ = std::any::type_name::<RagIndexState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  304 — RAG Citation Extraction (VilApp)                    ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Post-process LLM output to extract [DocN] refs    ║");
    println!("║          Build structured citations: section, title, snippet║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Legal docs indexed: {} clauses", LEGAL_DOCS.len());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    // Display authentication mode (API key vs simulator)
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode" } else { "OPENAI_API_KEY" });
    // Display the endpoint URL for this service
    println!("  Listening on http://localhost:3113/api/cited-rag");
    // Display the upstream data source URL
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("rag-citation")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/cited-rag", post(cited_rag_handler));

    VilApp::new("rag-citation-extraction")
        .port(3113)
        .service(svc)
        .run()
        .await;
}
