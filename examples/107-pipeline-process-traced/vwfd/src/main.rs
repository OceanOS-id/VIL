// 107 — Supply Chain Process Traced (VWFD)
// Business logic identical to standard:
//   Upstream: SSE from credit-sim :18081 /credits/stream
//   Transform: add _traced=true, _hop="carrier_handoff", _supply_chain="PKG pipeline v2"
use serde_json::{json, Value};

fn trace_records(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());
    let traced: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();
            obj.insert("_traced".into(), json!(true));
            obj.insert("_hop".into(), json!("carrier_handoff"));
            obj.insert("_supply_chain".into(), json!("PKG pipeline v2"));
            r
        }).collect(),
        None => vec![],
    };
    Ok(json!({"total": traced.len(), "records": traced}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/107-pipeline-process-traced/vwfd/workflows", 3307)
        .native("trace_records", trace_records)
        .run().await;
}
