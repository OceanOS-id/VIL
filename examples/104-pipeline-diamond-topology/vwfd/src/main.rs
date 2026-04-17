// 104 — Diamond Topology: Summary + Detail Views (VWFD)
// Business logic identical to standard:
//   Summary (/diamond): NPL-only (kol>=3), compact fields + _view=SUMMARY + _npl_class
//   Detail (/diamond-detail): all records + _risk_score + _risk_class + _ltv_pct + _view=DETAIL
use serde_json::{json, Value};

fn summary_view(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());
    let filtered: Vec<Value> = match records {
        Some(arr) => arr.iter().filter_map(|rec| {
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            if kol >= 3 {
                let npl_class = match kol {
                    3 => "KURANG_LANCAR", 4 => "DIRAGUKAN", 5 => "MACET", _ => "NPL_OTHER"
                };
                Some(json!({
                    "id": rec["id"], "nik": rec["nik"], "nama": rec["nama_lengkap"],
                    "kol": kol, "saldo": rec["saldo_outstanding"],
                    "_view": "SUMMARY", "_npl_class": npl_class
                }))
            } else { None }
        }).collect(),
        None => vec![],
    };
    Ok(json!({"total": filtered.len(), "records": filtered}))
}

fn detail_view(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());
    let enriched: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            let saldo = rec["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let plafon = rec["jumlah_kredit"].as_f64().unwrap_or(1.0);
            let risk_score = kol as f64 * 20.0 + saldo / 1_000_000.0;
            obj.insert("_view".into(), json!("DETAIL"));
            obj.insert("_risk_score".into(), json!((risk_score * 100.0).round() / 100.0));
            obj.insert("_risk_class".into(), json!(
                if risk_score > 100.0 { "HIGH" } else if risk_score > 50.0 { "MEDIUM" } else { "LOW" }
            ));
            obj.insert("_ltv_pct".into(), json!(
                if plafon > 0.0 { ((saldo / plafon * 100.0) * 100.0).round() / 100.0 } else { 0.0 }
            ));
            r
        }).collect(),
        None => vec![],
    };
    Ok(json!({"total": enriched.len(), "records": enriched}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/104-pipeline-diamond-topology/vwfd/workflows", 3206)
        .native("summary_view", summary_view)
        .native("detail_view", detail_view)
        .run().await;
}
