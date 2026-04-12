// 901 — KYC Onboarding (FaaS Demo: vil_phone, vil_email_validate, vil_hash, vil_id_gen, vil_mask)
//
// Demonstrates 5 built-in FaaS functions in V-CEL expressions:
//   parse_phone()    — validate & format phone number
//   validate_email() — check email format
//   sha256()         — hash sensitive data for audit
//   uuid_v4()        — generate application ID
//   mask_pii()       — mask PII for safe logging
//
// All processing via V-CEL expressions in YAML — zero NativeCode handlers.

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/901-faas-kyc-onboarding/vwfd/workflows", 8080)
        .run()
        .await;
}
