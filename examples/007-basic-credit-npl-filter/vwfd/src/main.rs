// 007 — NPL Filter (NDJSON stream, kolektabilitas >= 3)
// Business logic matches standard src/main.rs:
//   - Stream credit records from Core Banking
//   - Filter: only records with kolektabilitas >= 3 (NPL per OJK)
//     3 = Kurang Lancar (Substandard)
//     4 = Diragukan (Doubtful)
//     5 = Macet (Loss)
use serde_json::{json, Value};

fn filter_npl_records(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());

    let npl: Vec<&Value> = match records {
        Some(arr) => arr.iter().filter(|rec| {
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            kol >= 3
        }).collect(),
        None => vec![],
    };

    Ok(json!({
        "total_npl": npl.len(),
        "records": npl
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/007-basic-credit-npl-filter/vwfd/workflows", 3081)
        .native("filter_npl_records", filter_npl_records)
        .run().await;
}
