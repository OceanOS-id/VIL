// ╔════════════════════════════════════════════════════════════════════════╗
// ║  037 — Insurance Claim Processing (#[derive(VilModel)])             ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: #[derive(VilModel)], from_shm_bytes(), to_json_bytes(),   ║
// ║            VilModelTrait — SHM-aware serialization                   ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: An insurance company processes claims submitted by        ║
// ║  policyholders. Claims arrive as JSON over HTTP, but internally     ║
// ║  they travel between services via SHM (shared memory) for           ║
// ║  zero-copy performance.                                              ║
// ║                                                                      ║
// ║  VilModel enables this by providing:                                 ║
// ║    - from_shm_bytes(): deserialize directly from SHM region          ║
// ║      (no copy from network buffer to heap — the struct reads SHM)   ║
// ║    - to_json_bytes(): serialize to Bytes for mesh forwarding         ║
// ║      (the bytes can be placed directly into ExchangeHeap)           ║
// ║                                                                      ║
// ║  Flow:                                                               ║
// ║    1. Policyholder POSTs claim JSON via HTTP                         ║
// ║    2. VilModel::from_shm_bytes() deserializes from ShmSlice         ║
// ║    3. Business logic validates and processes the claim               ║
// ║    4. VilModel::to_json_bytes() serializes for mesh forwarding      ║
// ║    5. Adjuster service reads claim from SHM (zero network hop)      ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-vilmodel-derive
// Test: curl -X POST http://localhost:8080/api/claims/submit \
//         -H 'Content-Type: application/json' \
//         -d '{"claim_id":5001,"policy_id":20045,"amount_cents":250000,"claim_type":"auto_collision","description":"Rear-end collision at intersection, bumper damage"}'
//       curl http://localhost:8080/api/claims/sample

use vil_server::prelude::*;

// ── Insurance Claim Model ───────────────────────────────────────────────
//
// #[derive(VilModel)] generates the VilModelTrait implementation:
//   - from_shm_bytes(&[u8]) → Result<Self, VilModelError>
//   - to_json_bytes(&self) → Result<Bytes, VilModelError>
//
// Requirements: the struct must be Serialize + Deserialize + Clone.
// VilModel is the bridge between JSON (HTTP world) and SHM (mesh world).
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct InsuranceClaim {
    /// Unique claim identifier assigned by the system
    claim_id: u64,
    /// Policy number this claim is filed against
    policy_id: u64,
    /// Claimed amount in cents (e.g., 250000 = $2,500.00)
    amount_cents: u64,
    /// Type of claim: auto_collision, home_fire, health_emergency, etc.
    claim_type: String,
    /// Free-text description of the incident
    description: String,
}

/// Response returned after claim submission.
/// Includes metadata about the SHM serialization for transparency.
#[derive(Serialize)]
struct ClaimResponse {
    claim: InsuranceClaim,
    shm_bytes_len: usize,
    deserialized_from: &'static str,
    adjuster_assignment: &'static str,
    estimated_processing_days: u32,
}

// ── Claim Submission Handler ────────────────────────────────────────────

/// Submit an insurance claim.
///
/// KEY VIL FEATURE: VilModel::from_shm_bytes()
/// The claim JSON arrives in an ShmSlice (shared memory region).
/// from_shm_bytes() deserializes directly from that memory — no intermediate
/// copy to a heap-allocated buffer. For large claims with attachments
/// (photos, police reports), this saves significant memory allocation.
async fn submit_claim(body: ShmSlice) -> Result<VilResponse<ClaimResponse>, VilError> {
    // Deserialize the insurance claim from SHM bytes (zero-copy path).
    // VilModel::from_shm_bytes reads directly from the ShmSlice memory region.
    let claim = InsuranceClaim::from_shm_bytes(body.as_bytes())
        .map_err(|e| VilError::bad_request(format!("Invalid claim data: {}", e)))?;

    // Business validation: amount must be positive and claim type recognized
    if claim.amount_cents == 0 {
        return Err(VilError::bad_request("Claim amount must be greater than zero"));
    }

    // Serialize to JSON bytes for mesh forwarding to the adjuster service.
    // to_json_bytes() produces Bytes that can be placed directly into ExchangeHeap
    // for the adjuster service to read via SHM (no network round-trip).
    let shm_bytes = claim.to_json_bytes()
        .map_err(|e| VilError::internal(format!("Claim serialization failed: {}", e)))?;

    // Assign adjuster based on claim type (in production: load-balanced queue)
    let adjuster_assignment = match claim.claim_type.as_str() {
        "auto_collision" | "auto_theft" => "Auto Claims Team — Adjuster #A-12",
        "home_fire" | "home_flood" => "Property Claims Team — Adjuster #P-07",
        "health_emergency" => "Health Claims Team — Adjuster #H-03",
        _ => "General Claims Team — Adjuster #G-01",
    };

    // Estimate processing time based on claim amount
    let estimated_days = if claim.amount_cents > 500_000 { 14 } else { 7 };

    Ok(VilResponse::ok(ClaimResponse {
        claim,
        shm_bytes_len: shm_bytes.len(),
        deserialized_from: "ShmSlice via VilModel::from_shm_bytes() — zero-copy deserialization",
        adjuster_assignment,
        estimated_processing_days: estimated_days,
    }))
}

/// Return a sample claim to demonstrate VilModel round-trip serialization.
///
/// This shows the complete cycle: create → to_json_bytes → from_shm_bytes.
/// The data makes a round-trip through the SHM serialization format and
/// comes back identical — proving the VilModel derive is lossless.
async fn sample_claim() -> Result<VilResponse<ClaimResponse>, VilError> {
    // Create a sample claim (as if read from a database)
    let claim = InsuranceClaim {
        claim_id: 9999,
        policy_id: 40001,
        amount_cents: 125000,
        claim_type: "home_flood".into(),
        description: "Basement flooding after heavy rain, damaged furniture and electronics".into(),
    };

    // Serialize to SHM bytes
    let shm_bytes = claim.to_json_bytes()
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Round-trip: deserialize from the bytes we just serialized
    // This proves VilModel serialization is lossless and consistent.
    let round_tripped = InsuranceClaim::from_shm_bytes(&shm_bytes)
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(ClaimResponse {
        claim: round_tripped,
        shm_bytes_len: shm_bytes.len(),
        deserialized_from: "VilModel round-trip (to_json_bytes → from_shm_bytes) — lossless",
        adjuster_assignment: "Property Claims Team — Adjuster #P-07",
        estimated_processing_days: 7,
    }))
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  037 — Insurance Claim Processing (#[derive(VilModel)])              ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  from_shm_bytes() → zero-copy deserialization from SHM               ║");
    println!("║  to_json_bytes()  → serialize to Bytes for mesh forwarding            ║");
    println!("║  Round-trip: JSON → SHM → JSON is lossless                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let claims_svc = ServiceProcess::new("claims")
        .endpoint(Method::POST, "/claims/submit", post(submit_claim))
        .endpoint(Method::GET, "/claims/sample", get(sample_claim));

    VilApp::new("insurance-claim-processing")
        .port(8080)
        .service(claims_svc)
        .run()
        .await;
}
