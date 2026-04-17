// 005 — Core Banking Data Ingestion (NDJSON stream + risk enrichment)
// Business logic matches standard src/main.rs:
//   - Fetch NDJSON from Core Banking Simulator
//   - Enrich each record: _risk_category (OJK kolektabilitas), _ltv_ratio
use serde_json::{json, Value};

fn enrich_credit_records(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());

    let enriched: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();

            // _risk_category based on kolektabilitas (OJK regulation)
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            let risk = match kol {
                1 => "LANCAR",
                2 => "DALAM_PERHATIAN_KHUSUS",
                3 => "KURANG_LANCAR",
                4 => "DIRAGUKAN",
                5 => "MACET",
                _ => "UNKNOWN",
            };
            obj.insert("_risk_category".into(), json!(risk));

            // _ltv_ratio = saldo_outstanding / jumlah_kredit * 100
            let saldo = rec["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let kredit = rec["jumlah_kredit"].as_f64().unwrap_or(1.0);
            let ltv = if kredit > 0.0 {
                ((saldo / kredit * 100.0) * 100.0).round() / 100.0
            } else { 0.0 };
            obj.insert("_ltv_ratio".into(), json!(ltv));

            r
        }).collect(),
        None => vec![],
    };

    Ok(json!({
        "total_records": enriched.len(),
        "records": enriched
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/005-basic-multiservice-mesh-ndjson/vwfd/workflows", 3084)
        .native("enrich_credit_records", enrich_credit_records)
        .run().await;
}
