// ╔════════════════════════════════════════════════════════════╗
// ║  305 — Healthcare AI with Safety Guardrails               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ║  Domain:   Medical Q&A with PII redaction, hallucination  ║
// ║            detection, and confidence scoring               ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p rag-plugin-usage-medical-qa
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What are the symptoms of diabetes?"}' \
//     http://localhost:3114/api/safe-rag
//
// BUSINESS CONTEXT:
//   Healthcare AI assistant for a hospital patient portal. Patients ask
//   medical questions and receive evidence-based answers from published
//   guidelines (ADA, ACC/AHA, CDC). Safety is paramount in healthcare AI:
//     - PII redaction prevents accidental exposure of patient data (HIPAA)
//     - Hallucination detection flags unsupported medical claims
//     - Confidence scoring helps clinicians assess answer reliability
//     - BLOCKED status prevents dangerous misinformation from reaching patients
//   Every response includes a mandatory disclaimer per FDA guidance on
//   clinical decision support software.
//
// HOW THIS DIFFERS FROM 301:
//   301 = RAG -> LLM -> return as-is
//   305 = RAG -> LLM -> GUARDRAIL PIPELINE:
//         1. PII detection (email, phone, NIK regex)
//         2. Hallucination marker check
//         3. Redact if PII found
//         4. Add guardrail_status: PASS / REDACTED / BLOCKED
//         5. Add confidence_score

use vil_server::prelude::*;
use vil_rag::semantic::{RagQueryEvent, RagIngestEvent, RagFault, RagIndexState};

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct GuardrailState {
    pub total_queries: u64,
    pub pass_count: u64,
    pub redacted_count: u64,
    pub blocked_count: u64,
    pub pii_detections: u64,
}

#[derive(Clone, Debug)]
pub struct GuardrailCheckEvent {
    pub query: String,
    pub status: String,
    pub pii_types_found: Vec<String>,
    pub hallucination_score: f64,
    pub confidence: f64,
}

#[vil_fault]
pub enum GuardrailFault {
    PiiDetected,
    HallucinationDetected,
    ResponseBlocked,
    GuardrailTimeout,
}

// ── Medical Knowledge Base ──────────────────────────────────────────

const MEDICAL_DOCS: &[(&str, &str)] = &[
    ("[Doc1]", "DISCLAIMER: This system provides information based on published medical \
               guidelines for educational purposes only. NOT medical advice. Always consult \
               a qualified healthcare professional."),
    ("[Doc2]", "Type 2 Diabetes — Screening (ADA 2024): Recommended for adults aged 35+ \
               or BMI >= 25 with risk factors. Diagnostic criteria: fasting glucose >= 126 \
               mg/dL, HbA1c >= 6.5%, or 2-hour glucose >= 200 mg/dL. Common symptoms: \
               increased thirst, frequent urination, fatigue, blurred vision."),
    ("[Doc3]", "Hypertension Management (ACC/AHA 2023): BP target < 130/80 mmHg. First-line: \
               thiazide diuretics, ACE inhibitors, ARBs, or calcium channel blockers. \
               Lifestyle: DASH diet, sodium < 2300 mg/day, exercise 150 min/week."),
    ("[Doc4]", "Common Cold vs Flu (CDC 2024): Cold symptoms are gradual onset, mild fever, \
               runny nose, sore throat. Flu symptoms are sudden onset, high fever (100-104F), \
               body aches, severe fatigue. Seek emergency care for difficulty breathing, \
               persistent chest pain, or confusion."),
];

// ── Guardrail Functions ─────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PiiDetection {
    pii_type: String,
    pattern: String,
    redacted: bool,
}

/// Check for PII patterns and return detections + redacted text.
/// Business: HIPAA requires that any patient-identifiable data be masked
/// before display. This catches emails, phone numbers, and Indonesian NIK
/// (national ID) that might leak through LLM outputs.
fn check_and_redact_pii(text: &str) -> (String, Vec<PiiDetection>) {
    let mut detections = Vec::new();
    let mut redacted = text.to_string();

    // Email pattern: simple check for word@word.word
    let email_indicators: Vec<&str> = text.split_whitespace()
        .filter(|w| w.contains('@') && w.contains('.'))
        .collect();
    for email in &email_indicators {
        detections.push(PiiDetection {
            pii_type: "email".into(),
            pattern: email.to_string(),
            redacted: true,
        });
        redacted = redacted.replace(email, "[EMAIL_REDACTED]");
    }

    // Phone pattern: sequences of 10+ digits (with optional separators)
    let digits_only: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    // Check for phone-like sequences in original text
    let mut phone_buf = String::new();
    let mut in_phone = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() || (in_phone && (ch == '-' || ch == ' ' || ch == '+' || ch == '(' || ch == ')')) {
            phone_buf.push(ch);
            in_phone = true;
        } else {
            if phone_buf.chars().filter(|c| c.is_ascii_digit()).count() >= 10 {
                detections.push(PiiDetection {
                    pii_type: "phone".into(),
                    pattern: phone_buf.clone(),
                    redacted: true,
                });
                redacted = redacted.replace(&phone_buf, "[PHONE_REDACTED]");
            }
            phone_buf.clear();
            in_phone = false;
        }
    }
    // Check remaining buffer
    if phone_buf.chars().filter(|c| c.is_ascii_digit()).count() >= 10 {
        detections.push(PiiDetection {
            pii_type: "phone".into(),
            pattern: phone_buf.clone(),
            redacted: true,
        });
        redacted = redacted.replace(&phone_buf, "[PHONE_REDACTED]");
    }

    // NIK (Indonesian ID) pattern: exactly 16 digits
    if digits_only.len() >= 16 {
        // Check for 16-digit sequences in text
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in &words {
            let d: String = word.chars().filter(|c| c.is_ascii_digit()).collect();
            if d.len() == 16 {
                detections.push(PiiDetection {
                    pii_type: "nik".into(),
                    pattern: format!("{}...", &d[..6]),
                    redacted: true,
                });
                redacted = redacted.replace(word, "[NIK_REDACTED]");
            }
        }
    }

    (redacted, detections)
}

/// Check for hallucination markers in the response.
/// Business: in medical AI, hallucinations can cause patient harm.
/// The scoring system flags hedging language, uncited claims, and
/// exaggerated promises ("proven cure") that indicate fabricated content.
fn check_hallucination(text: &str) -> (f64, Vec<String>) {
    let mut markers = Vec::new();
    let lower = text.to_lowercase();

    // Hallucination indicators
    if lower.contains("i think") || lower.contains("i believe") {
        markers.push("hedging_language".into());
    }
    if lower.contains("as an ai") || lower.contains("i cannot") || lower.contains("i don't have") {
        markers.push("ai_self_reference".into());
    }
    if lower.contains("studies show") && !lower.contains("[doc") {
        markers.push("uncited_claim".into());
    }
    if lower.contains("always") || lower.contains("never") || lower.contains("guaranteed") {
        markers.push("absolute_language".into());
    }
    if lower.contains("100%") || lower.contains("proven cure") || lower.contains("miracle") {
        markers.push("exaggerated_claim".into());
    }

    // Score: 0.0 = no hallucination, 1.0 = high hallucination risk
    let score = (markers.len() as f64 * 0.25).min(1.0);
    (score, markers)
}

/// Compute confidence score based on guardrail results
fn compute_confidence(pii_count: usize, hallucination_score: f64, answer_len: usize) -> f64 {
    let mut confidence = 1.0;
    if pii_count > 0 { confidence -= 0.3; }
    confidence -= hallucination_score * 0.4;
    if answer_len < 20 { confidence -= 0.2; }  // Too short = likely low quality
    confidence.max(0.0).min(1.0)
}

// ── Request / Response ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SafeRagRequest {
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel)]
struct SafeRagResponse {
    content: String,
    guardrail_status: String,     // "PASS", "REDACTED", "BLOCKED"
    confidence_score: f64,
    pii_detections: Vec<PiiDetection>,
    hallucination_markers: Vec<String>,
    hallucination_score: f64,
    disclaimer: String,
}

// ── Handler ──────────────────────────────────────────────────────────

async fn safe_rag_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<SafeRagResponse>> {
    let req: SafeRagRequest = body.json().expect("invalid JSON body");
    // Step 1: Build RAG context
    let context: String = MEDICAL_DOCS.iter()
        .map(|(id, content)| format!("{} {}", id, content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = format!(
        "You are a medical information assistant. Answer using ONLY the guidelines below. \
         Cite as [DocN]. Include the disclaimer. Do NOT provide personal medical advice.\n\n\
         Guidelines:\n{}",
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

    let raw_answer = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Step 2: GUARDRAIL PIPELINE
    // 2a. PII check + redaction
    let (redacted_answer, pii_detections) = check_and_redact_pii(&raw_answer);

    // 2b. Hallucination check
    let (hallucination_score, hallucination_markers) = check_hallucination(&raw_answer);

    // 2c. Determine guardrail status.
    // Decision matrix: >= 0.75 hallucination = BLOCKED (too risky for patients)
    //                  PII found or >= 0.25 = REDACTED (safe after cleanup)
    //                  otherwise = PASS (clean answer)
    let guardrail_status = if hallucination_score >= 0.75 {
        "BLOCKED".to_string()
    } else if !pii_detections.is_empty() || hallucination_score >= 0.25 {
        "REDACTED".to_string()
    } else {
        "PASS".to_string()
    };

    // 2d. Compute confidence
    let confidence_score = compute_confidence(
        pii_detections.len(),
        hallucination_score,
        raw_answer.len(),
    );
    let confidence_score = (confidence_score * 100.0).round() / 100.0;

    // 2e. Choose final content
    let content = if guardrail_status == "BLOCKED" {
        "Response blocked by guardrail. The generated answer contained too many \
         hallucination markers and cannot be safely returned.".to_string()
    } else {
        redacted_answer
    };

    // Semantic audit
    let _event = RagQueryEvent {
        question: req.prompt,
        chunks_retrieved: MEDICAL_DOCS.len() as u32,
        answer_length: content.len() as u32,
        latency_ms: 0,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(SafeRagResponse {
        content,
        guardrail_status,
        confidence_score,
        pii_detections,
        hallucination_markers,
        hallucination_score: (hallucination_score * 100.0).round() / 100.0,
        disclaimer: "For educational purposes only. NOT medical advice. \
                      Consult a qualified healthcare professional."
            .to_string(),
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
    println!("║  305 — RAG Guardrail Pipeline (VilApp)                     ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Post-LLM guardrails: PII redaction, hallucination ║");
    println!("║          detection, confidence scoring, PASS/REDACTED/BLOCK║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  DISCLAIMER: Medical demo only. NOT medical advice.");
    println!("  Guardrail checks:");
    println!("    - PII: email, phone (10+ digits), NIK (16 digits)");
    println!("    - Hallucination: hedging, AI self-ref, uncited claims");
    println!("    - Status: PASS / REDACTED / BLOCKED");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode" } else { "OPENAI_API_KEY" });
    println!("  Listening on http://localhost:3114/api/safe-rag");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    let svc = ServiceProcess::new("rag-guardrail")
        .emits::<RagQueryEvent>()
        .faults::<RagFault>()
        .manages::<RagIndexState>()
        .prefix("/api")
        .endpoint(Method::POST, "/safe-rag", post(safe_rag_handler));

    VilApp::new("rag-guardrail-pipeline")
        .port(3114)
        .service(svc)
        .run()
        .await;
}
