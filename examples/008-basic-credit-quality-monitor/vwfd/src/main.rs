// 008 — Credit Quality Monitor (Java Sidecar + Native)
use serde_json::{json, Value};
fn cross_reference_check(input: &Value) -> Result<Value, String> {
    let nik = input["nik"].as_str().unwrap_or("");
    let kol = input["kolektabilitas"].as_u64().unwrap_or(1);
    Ok(json!({"nik": nik, "cross_ref_flag": kol >= 3, "kolektabilitas": kol, "source": "SLIK"}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/008-basic-credit-quality-monitor/vwfd/workflows", 3108)
        .sidecar("validate_credit_schema", "java -cp examples/008-basic-credit-quality-monitor/vwfd/sidecar/java CreditSchemaValidator")
        .native("cross_reference_check", cross_reference_check)
        .run().await;
}
