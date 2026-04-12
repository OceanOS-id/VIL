// 901 — KYC Onboarding (Standard Pattern)
// Demonstrates: vil_phone, vil_email_validate, vil_hash, vil_id_gen, vil_mask
use serde_json::{json, Value};

fn kyc_verify(name: &str, phone: &str, email: &str) -> Result<Value, String> {
    let app_id = vil_id_gen::uuid_v4(&[])?;
    let phone_result = vil_phone::parse_phone(&[Value::String(phone.into()), Value::String("ID".into())])?;
    let email_result = vil_email_validate::validate_email(&[Value::String(email.into())])?;
    let email_hash = vil_hash::sha256(&[Value::String(email.into())])?;
    let phone_masked = vil_mask::mask_pii(&[Value::String(phone.into()), Value::String("phone".into())])?;

    Ok(json!({
        "application_id": app_id,
        "name": name,
        "phone_valid": phone_result["valid"],
        "phone_masked": phone_masked,
        "phone_e164": phone_result["e164"],
        "email_valid": email_result["valid"],
        "email_domain": email_result["domain"],
        "email_hash": email_hash,
        "status": if phone_result["valid"] == true && email_result["valid"] == true { "approved" } else { "needs_review" }
    }))
}

fn main() {
    println!("=== 901 KYC Onboarding ===");
    match kyc_verify("Alice", "081234567890", "alice@example.com") {
        Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
