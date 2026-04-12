// 904 — Financial Analytics (Standard Pattern)
// Demonstrates: vil_datefmt, vil_duration, vil_anomaly, vil_geodist, vil_stats
use serde_json::{json, Value};

fn main() {
    // 1. Parse and format date
    let parsed = vil_datefmt::parse_date(&[Value::String("2024-06-15".into())]).unwrap();
    println!("Parsed date: {}", parsed);

    // 2. Current time
    let now = vil_datefmt::now(&[]).unwrap();
    println!("Now: {}", now);

    // 3. Calculate customer age
    let age = vil_duration::age(&[Value::String("1990-05-15".into())]).unwrap();
    println!("Customer age: {} years", age);

    // 4. Calculate loan duration
    let dur = vil_duration::duration(&[
        Value::String("2024-01-01".into()),
        Value::String("2026-06-30".into()),
        Value::String("months".into()),
    ]).unwrap();
    println!("Loan duration: {:.1} months", dur);

    // 5. Transaction statistics
    let amounts = json!([150000, 200000, 175000, 180000, 190000, 50000000]);
    let avg = vil_stats::mean(&[amounts.clone()]).unwrap();
    let p95 = vil_stats::percentile(&[amounts.clone(), json!(95)]).unwrap();
    println!("Avg transaction: {}, P95: {}", avg, p95);

    // 6. Anomaly detection on latest transaction
    let history = json!([150000, 200000, 175000, 180000, 190000]);
    let check = vil_anomaly::is_anomaly(&[json!(50000000), history, Value::String("zscore".into()), json!(2.0)]).unwrap();
    println!("Anomaly: {} (z={})", check["anomaly"], check["score"]);

    // 7. Distance between branches
    let dist = vil_geodist::geo_distance(&[
        json!(-6.2088), json!(106.8456),  // Jakarta
        json!(-7.7956), json!(110.3695),  // Yogyakarta
        Value::String("km".into()),
    ]).unwrap();
    println!("Jakarta -> Yogya: {} km", dist["distance"]);
}
