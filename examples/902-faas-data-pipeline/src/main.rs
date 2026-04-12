// 902 — Data Pipeline (Standard Pattern)
// Demonstrates: vil_parse_csv, vil_parse_xml, vil_validate_schema, vil_reshape, vil_stats
use serde_json::{json, Value};

fn main() {
    // 1. Parse CSV data
    let csv_data = "name,age,score\nAlice,30,95.5\nBob,25,87.3\nCharlie,35,92.1";
    let parsed = vil_parse_csv::parse_csv(&[Value::String(csv_data.into())]).unwrap();
    println!("CSV parsed: {} rows", parsed["count"]);

    // 2. Parse XML data
    let xml_data = "<users><user id=\"1\"><name>Alice</name><role>admin</role></user></users>";
    let xml = vil_parse_xml::parse_xml(&[Value::String(xml_data.into())]).unwrap();
    println!("XML parsed: {} elements", xml["count"]);

    // 3. Validate against JSON Schema
    let schema = json!({"type": "object", "required": ["name", "age"], "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}});
    let data = json!({"name": "Alice", "age": 30});
    let valid = vil_validate_schema::validate_schema(&[data, schema]).unwrap();
    println!("Schema valid: {}", valid["valid"]);

    // 4. Reshape data
    let input = json!({"user": {"profile": {"first_name": "Alice", "contact": {"email": "alice@example.com"}}}});
    let mapping = json!({"name": "user.profile.first_name", "email": "user.profile.contact.email"});
    let reshaped = vil_reshape::reshape(&[input, mapping]).unwrap();
    println!("Reshaped: {}", reshaped);

    // 5. Calculate statistics
    let scores = json!([95.5, 87.3, 92.1, 88.7, 91.0]);
    let avg = vil_stats::mean(&[scores.clone()]).unwrap();
    let med = vil_stats::median(&[scores.clone()]).unwrap();
    let std = vil_stats::stdev(&[scores]).unwrap();
    println!("Stats: mean={}, median={}, stdev={}", avg, med, std);
}
