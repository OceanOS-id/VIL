// 046 — Flight Search Scatter-Gather (NativeCode — mesh routing is native infra)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/046-basic-mesh-scatter-gather/vwfd/workflows", 8080)
        .native("mesh_flight_scatter", |input| {
            let body = &input["body"];
            let origin = body["origin"].as_str().unwrap_or("CGK");
            let dest = body["destination"].as_str().unwrap_or("SIN");
            Ok(json!({
                "results": [
                    {"airline": "GA", "price": 1250000, "origin": origin, "destination": dest},
                    {"airline": "SQ", "price": 2100000, "origin": origin, "destination": dest},
                    {"airline": "QZ", "price": 890000, "origin": origin, "destination": dest},
                ],
                "total_results": 3,
            }))
        })
        .run().await;
}
