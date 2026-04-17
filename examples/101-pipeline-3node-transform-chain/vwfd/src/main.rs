// 101 — 3-Node ETL Transform Chain (VWFD)
// Business logic identical to standard:
//   - Upstream: NDJSON from credit-sim :18081 (100 records)
//   - Step 1: Normalize — uppercase nama_lengkap
//   - Step 2: Enrich — _risk_score = kol * 20 + saldo / 1_000_000
//   - Step 3: Classify — _risk_class: HIGH (>100), MEDIUM (>50), LOW
use serde_json::{json, Value};

fn etl_transform_chain(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());

    let transformed: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();

            // Step 1: Normalize — uppercase nama_lengkap
            if let Some(nama) = rec["nama_lengkap"].as_str() {
                obj.insert("nama_lengkap".into(), json!(nama.to_uppercase()));
            }

            // Step 2: Enrich — risk_score = kol * 20 + saldo / 1_000_000
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            let saldo = rec["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let risk_score = kol as f64 * 20.0 + saldo / 1_000_000.0;
            obj.insert("_risk_score".into(), json!((risk_score * 100.0).round() / 100.0));

            // Step 3: Classify
            let risk_class = if risk_score > 100.0 { "HIGH" }
                else if risk_score > 50.0 { "MEDIUM" }
                else { "LOW" };
            obj.insert("_risk_class".into(), json!(risk_class));

            r
        }).collect(),
        None => vec![],
    };

    Ok(json!({
        "total_records": transformed.len(),
        "records": transformed
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/101-pipeline-3node-transform-chain/vwfd/workflows", 3203)
        .native("etl_transform_chain", etl_transform_chain)
        .run().await;
}
