// 903 — Secure API Gateway (Standard Pattern)
// Demonstrates: vil_crypto, vil_jwt, vil_webhook_out, vil_regex, vil_template
use serde_json::{json, Value};

fn main() {
    // 1. Generate JWT token
    let payload = json!({"sub": "user-123", "role": "admin", "exp": 9999999999u64});
    let token = vil_jwt::jwt_sign(&[payload.clone(), Value::String("my-secret-key".into())]).unwrap();
    println!("JWT: {}", token);

    // 2. Verify JWT
    let decoded = vil_jwt::jwt_verify(&[token.clone(), Value::String("my-secret-key".into())]).unwrap();
    println!("Decoded: sub={}", decoded["sub"]);

    // 3. Encrypt sensitive data
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"; // 64 hex = 32 bytes
    let encrypted = vil_crypto::aes_encrypt(&[Value::String("sensitive-data".into()), Value::String(key.into())]).unwrap();
    println!("Encrypted: {}", encrypted);

    // 4. Decrypt
    let decrypted = vil_crypto::aes_decrypt(&[encrypted, Value::String(key.into())]).unwrap();
    println!("Decrypted: {}", decrypted);

    // 5. Regex validation
    let is_valid = vil_regex::regex_match(&[
        Value::String("user@example.com".into()),
        Value::String(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".into()),
    ]).unwrap();
    println!("Email regex valid: {}", is_valid);

    // 6. Template rendering
    let rendered = vil_template::render_template(&[
        Value::String("Hello {{name}}, your role is {{role}}.".into()),
        json!({"name": "Alice", "role": "admin"}),
    ]).unwrap();
    println!("Template: {}", rendered);
}
