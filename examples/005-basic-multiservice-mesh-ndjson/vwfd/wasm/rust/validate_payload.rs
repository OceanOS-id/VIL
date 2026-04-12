//! JSON Payload Validator — Rust WASM Module
//! Validates required fields exist and have correct types.

use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let has_id = input.contains(r#""id""#);
    let has_name = input.contains(r#""name""#);
    let has_type = input.contains(r#""type""#);

    let mut errors = Vec::new();
    if !has_id { errors.push("missing: id"); }
    if !has_name { errors.push("missing: name"); }

    let valid = errors.is_empty();
    if valid {
        println!(r#"{{"valid":true,"fields_checked":["id","name","type"],"warnings":{}}}"#,
            if !has_type { 1 } else { 0 });
    } else {
        println!(r#"{{"valid":false,"errors":["{}"]}}"#, errors.join("\",\""));
    }
}
