// 102 — Fan-Out Scatter: NPL + Healthy (VWFD)
// Business logic identical to standard:
//   Pipeline A (NPL): kol >= 3 → keep, add _pipeline=NPL, _npl_class
//   Pipeline B (Healthy): kol < 3 → keep
// Standard uses 2 ports (3091 NPL, 3092 Healthy). VWFD single port with /npl path.
use serde_json::{json, Value};

fn npl_filter(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());
    let filtered: Vec<Value> = match records {
        Some(arr) => arr.iter().filter_map(|rec| {
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            if kol >= 3 {
                let mut r = rec.clone();
                let obj = r.as_object_mut().unwrap();
                obj.insert("_pipeline".into(), json!("NPL"));
                obj.insert("_npl_class".into(), json!(match kol {
                    3 => "KURANG_LANCAR", 4 => "DIRAGUKAN", 5 => "MACET", _ => "NPL_OTHER"
                }));
                Some(r)
            } else { None }
        }).collect(),
        None => vec![],
    };
    Ok(json!({"total": filtered.len(), "records": filtered}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/102-pipeline-fanout-scatter/vwfd/workflows", 3302)
        .native("npl_filter", npl_filter)
        .run().await;
}
