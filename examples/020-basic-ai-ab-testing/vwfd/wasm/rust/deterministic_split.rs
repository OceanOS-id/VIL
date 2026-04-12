//! A/B Deterministic Split — Rust WASM Module
//! Hash-based 80/20 traffic routing. Same input always gets same group.

use std::io::{self, Read};

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    // Extract prompt field for hashing
    let seed = if let Some(pos) = input.find(r#""prompt":""#) {
        let start = pos + 10;
        let end = input[start..].find('"').unwrap_or(0);
        &input[start..start + end]
    } else {
        &input
    };

    let hash = simple_hash(seed);
    let group = if hash % 100 < 80 { "A" } else { "B" };

    println!(r#"{{"group":"{}","hash":{},"split_ratio":"80/20"}}"#, group, hash);
}
