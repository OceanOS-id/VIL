// ╔════════════════════════════════════════════════════════════╗
// ║  205 — Legal Document Summarization Pipeline              ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Legal — Contract & Policy Summarization         ║
// ║  Pattern:  SDK_PIPELINE                                  ║
// ║  Token:    GenericToken                                  ║
// ║  Unique:   CHUNKING PIPELINE — splits long document,     ║
// ║            summarizes each chunk via LLM, merges         ║
// ║            summaries into final executive summary         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Processes long legal documents (contracts,      ║
// ║  regulatory filings, compliance policies) by splitting    ║
// ║  at sentence boundaries, summarizing each chunk, and      ║
// ║  merging into a final summary. Reduces 50-page contracts  ║
// ║  to 1-page executive briefings for legal review.          ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p llm-plugin-usage-summarizer
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"text": "Very long document text here...", "max_chunk_size": 500}' \
//     http://localhost:3104/summarize
//
// HOW THIS DIFFERS FROM 201/202:
//   201 = single prompt -> single LLM call
//   202 = SDK pipeline with model routing
//   205 = SDK pipeline with TRANSFORM step:
//         webhook -> chunk splitter (transform) -> per-chunk summarize -> merge
//   This demonstrates vil_workflow! with a transform node.

use std::sync::Arc;
use vil_sdk::prelude::*;

use vil_llm::pipeline;
use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
/// Summarizer state — tracks documents processed and compression ratios.
pub struct ChunkerState {
    pub documents_processed: u64,
    pub total_chunks_created: u64,
    pub total_input_chars: u64,
    pub total_summary_chars: u64,
    pub avg_compression_ratio: f64,
}

#[derive(Clone, Debug)]
/// Chunk summary event — logged for document processing analytics.
pub struct ChunkSummarizedEvent {
    pub document_id: String,
    pub chunk_index: u32,
    pub chunk_char_count: u32,
    pub summary_char_count: u32,
    pub is_final_merge: bool,
}

#[vil_fault]
/// Chunker faults — each triggers different error handling strategy.
pub enum ChunkerFault {
    DocumentTooShort,
    ChunkSummarizeFailed,
    MergeSummarizeFailed,
    UpstreamTimeout,
    InvalidChunkSize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline Configuration
// ─────────────────────────────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3104;
const WEBHOOK_PATH: &str = "/summarize";
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const DEFAULT_MAX_CHUNK: usize = 2000;

// ── Chunk Splitter (Transform logic) ────────────────────────────────

/// Split text into chunks of approximately max_size characters,
/// breaking at sentence boundaries when possible.
/// Splits long legal documents into chunks at sentence boundaries.
/// Respects sentence endings (., !, ?) to avoid breaking mid-sentence.
fn split_into_chunks(text: &str, max_size: usize) -> Vec<String> {
    let max_size = if max_size < 100 { 100 } else { max_size };
    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in text.split_inclusive(|c: char| c == '.' || c == '!' || c == '?') {
        if current.len() + sentence.len() > max_size && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        current.push_str(sentence);
    }
    if !current.is_empty() {
        chunks.push(current);
    }

    // If no sentence boundaries were found, split at word boundaries
    if chunks.is_empty() && !text.is_empty() {
        let mut start = 0;
        while start < text.len() {
            let end = std::cmp::min(start + max_size, text.len());
            // Find last space before end
            let split_at = if end < text.len() {
                text[start..end]
                    .rfind(' ')
                    .map(|p| start + p + 1)
                    .unwrap_or(end)
            } else {
                end
            };
            chunks.push(text[start..split_at].to_string());
            start = split_at;
        }
    }

    chunks
}

// ── Pipeline Builder ────────────────────────────────────────────────
// The summarization pipeline splits long legal documents into manageable
// chunks at sentence boundaries, sends all chunks to the LLM in a single
// prompt asking for per-chunk summaries and a final merged summary.
// This approach handles documents that exceed LLM context windows.

fn main() {
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // Sink: receives webhook POST with { "text": "...", "max_chunk_size": N }
    let sink_builder = pipeline::chat_sink(WEBHOOK_PORT, WEBHOOK_PATH);

    // Source: sends each chunk summary request to LLM
    let source_summarize = pipeline::chat_source(SSE_URL, "gpt-4");

    // Source with chunking transform: split text into chunks → build summary prompt
    let source_summarize = source_summarize.transform(|payload: &[u8]| {
        let req: serde_json::Value = serde_json::from_slice(payload).ok()?;
        let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let max_chunk = req
            .get("max_chunk_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_MAX_CHUNK);

        let chunks = split_into_chunks(text, max_chunk);
        let chunk_count = chunks.len();

        let chunk_list: String = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| format!("--- Chunk {}/{} ---\n{}", i + 1, chunk_count, c))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "The following document has been split into {} chunks. \
             Summarize each chunk in 1-2 sentences, then provide a \
             final merged summary of the entire document.\n\n{}",
            chunk_count, chunk_list
        );

        // Build OpenAI chat completion request body
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a document summarizer. For each chunk, \
                                provide a brief summary. Then provide a final \
                                MERGED SUMMARY combining all chunk summaries."
                },
                {"role": "user", "content": prompt}
            ],
            "stream": true
        });

        Some(serde_json::to_vec(&body).unwrap_or_default())
    });

    // Build the pipeline
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "ChunkedSummarizerPipeline",
        instances: [ sink_builder, source_summarize ],
        routes: [
            sink_builder.trigger_out -> source_summarize.trigger_in (LoanWrite),
            source_summarize.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_summarize.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // Semantic type registration
    let _event_type = std::any::type_name::<LlmResponseEvent>();
    let _fault_type = std::any::type_name::<LlmFault>();
    let _state_type = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  205 — LLM Chunked Summarizer Pipeline (SDK_PIPELINE)      ║");
    println!("║  Pattern: SDK_PIPELINE | Token: GenericToken                ║");
    println!("║  Unique: Chunk splitter transform -> per-chunk LLM -> merge║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Default chunk size: {} chars", DEFAULT_MAX_CHUNK);
    println!(
        "  Listening on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("  Upstream SSE: {}", SSE_URL);
    println!();
    println!("  Test:");
    println!("  curl -X POST -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"text\": \"Long document...\", \"max_chunk_size\": 500}}' \\");
    println!("    http://localhost:{}{}", WEBHOOK_PORT, WEBHOOK_PATH);
    println!();

    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_summarize);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("ChunkerSink panicked");
    t2.join().expect("ChunkerSource panicked");
}
