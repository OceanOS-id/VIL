// 037 — Insurance Claim Processing (Hybrid: Sidecar PHP for claim processing, NativeCode for sample)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/037-basic-vilmodel-derive/vwfd/workflows", 8080)
        // Sample claim — NativeCode (static demo data)
        .native("claims_sample_handler", |_| {
            let claim = json!({
                "claim_id": 1,
                "policy_id": 5001,
                "amount_cents": 150000,
                "claim_type": "auto",
                "description": "Sample claim for demonstration",
                "status": "pending"
            });
            let bytes_len = serde_json::to_vec(&claim).unwrap_or_default().len();
            Ok(json!({
                "claim": claim,
                "shm_bytes_len": bytes_len,
                "serialization": "VilModel zero-copy"
            }))
        })
        // Claim submission — Sidecar PHP (external PHP runtime)
        .sidecar("claim_processor", "php examples/037-basic-vilmodel-derive/vwfd/sidecar/php/claim_processor.php")
        .run()
        .await;
}
