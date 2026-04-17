// 103 — Fan-In Gather (VWFD)
// Business logic identical to standard:
//   Tag each record: _source=CORE_BANKING, _format=NDJSON, _is_delinquent=(kol>=3)
use serde_json::{json, Value};

fn fanin_gather(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());
    let tagged: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();
            obj.insert("_source".into(), json!("CORE_BANKING"));
            obj.insert("_format".into(), json!("NDJSON"));
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            obj.insert("_is_delinquent".into(), json!(kol >= 3));
            r
        }).collect(),
        None => vec![],
    };
    Ok(json!({"total": tagged.len(), "records": tagged}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/103-pipeline-fanin-gather/vwfd/workflows", 3303)
        .native("fanin_gather", fanin_gather)
        .run().await;
}
