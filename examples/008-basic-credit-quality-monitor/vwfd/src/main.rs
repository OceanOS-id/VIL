// 008 — Credit Data Quality Monitor (NDJSON stream + 5 validation rules)
// Business logic matches standard src/main.rs:
//   Rule 1: kolektabilitas must be 1-5
//   Rule 2: saldo_outstanding <= jumlah_kredit, both >= 0
//   Rule 3: NIK must be 16 chars
//   Rule 4: nama_lengkap must not be empty
//   Rule 5: _has_error flag (simulator dirty)
// Output: _quality_issues[], _quality_score (PASS/FAIL), _issue_count
use serde_json::{json, Value};

fn validate_quality(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());

    let validated: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            let mut r = rec.clone();
            let obj = r.as_object_mut().unwrap();
            let mut issues: Vec<&str> = Vec::new();

            // Rule 1: Kolektabilitas range (1-5)
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);
            if kol < 1 || kol > 5 {
                issues.push("invalid_kolektabilitas");
            }

            // Rule 2: Saldo vs Kredit
            let saldo = rec["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let kredit = rec["jumlah_kredit"].as_f64().unwrap_or(0.0);
            if saldo > kredit {
                issues.push("saldo_exceeds_kredit");
            }
            if saldo < 0.0 || kredit < 0.0 {
                issues.push("negative_amount");
            }

            // Rule 3: NIK format (16 digits)
            let nik = rec["nik"].as_str().unwrap_or("");
            if nik.len() != 16 {
                issues.push("invalid_nik_length");
            }

            // Rule 4: Name required
            let nama = rec["nama_lengkap"].as_str().unwrap_or("");
            if nama.is_empty() {
                issues.push("missing_nama");
            }

            // Rule 5: Simulator dirty flag
            let has_error = rec["_has_error"].as_bool().unwrap_or(false);
            if has_error {
                issues.push("simulator_dirty_flag");
            }
            let error_type = rec["_error_type"].as_str().unwrap_or("");

            let score = if issues.is_empty() { "PASS" } else { "FAIL" };
            obj.insert("_quality_issues".into(), json!(issues));
            obj.insert("_quality_score".into(), json!(score));
            obj.insert("_issue_count".into(), json!(issues.len()));
            if !error_type.is_empty() {
                obj.insert("_detected_error_type".into(), json!(error_type));
            }

            r
        }).collect(),
        None => vec![],
    };

    let pass_count = validated.iter().filter(|r| r["_quality_score"] == "PASS").count();
    let fail_count = validated.len() - pass_count;

    Ok(json!({
        "total_records": validated.len(),
        "pass_count": pass_count,
        "fail_count": fail_count,
        "records": validated
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/008-basic-credit-quality-monitor/vwfd/workflows", 3082)
        .native("validate_quality", validate_quality)
        .run().await;
}
