// ╔════════════════════════════════════════════════════════════════════════╗
// ║  032 — Payment Gateway HA (Failover + Retry Strategy)               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: VxFailoverConfig, FailoverStrategy::Retry                 ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: An e-commerce platform processes payments through a       ║
// ║  primary gateway (e.g., Stripe). If the primary gateway fails        ║
// ║  (timeout, 5xx, network error), VIL retries 3 times with backoff,   ║
// ║  then automatically fails over to a backup gateway (e.g., Adyen).   ║
// ║                                                                      ║
// ║  Why HA matters for payments:                                        ║
// ║    - Payment gateways have 99.95% uptime → still ~22 min/month down ║
// ║    - During Black Friday, even 30 seconds of downtime = lost revenue ║
// ║    - VIL's failover is declarative: one line of config, not hundreds ║
// ║      of lines of retry/circuit-breaker/fallback code                 ║
// ║                                                                      ║
// ║  Flow:                                                               ║
// ║    1. Customer submits payment → primary gateway                     ║
// ║    2. If primary fails → retry 3 times (exponential backoff)         ║
// ║    3. If all retries fail → failover to backup gateway               ║
// ║    4. Backup processes payment → customer sees no interruption       ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-failover-ha
// Test: curl http://localhost:8080/api/primary/health
//       curl http://localhost:8080/api/backup/health
//       curl -X POST http://localhost:8080/api/primary/charge \
//         -H 'Content-Type: application/json' \
//         -d '{"card_token":"tok_visa_4242","amount_cents":9999,"currency":"USD"}'

use vil_server::prelude::*;

// ── Payment Faults ──────────────────────────────────────────────────────
// These represent real failure modes when processing credit card payments.
// #[vil_fault] generates fault codes that VIL uses for retry/failover decisions.
#[vil_fault]
pub enum PaymentFault {
    /// Gateway did not respond within 5 seconds (network/infra issue)
    GatewayTimeout,
    /// Customer's card has insufficient balance
    InsufficientFunds,
    /// Card issuer declined the transaction (fraud, expired, blocked)
    CardDeclined,
    /// Backup gateway also failed — payment cannot be processed
    AllGatewaysDown,
}

// ── Business Domain Types ───────────────────────────────────────────────

/// Payment request from the customer checkout flow.
#[derive(Deserialize)]
struct PaymentRequest {
    card_token: String,
    amount_cents: u64,
    currency: String,
}

/// Payment result returned after successful charge.
#[derive(Serialize)]
struct PaymentResult {
    gateway: String,
    role: String,
    charge_id: String,
    amount_cents: u64,
    currency: String,
    status: &'static str,
    retry_strategy: String,
}

// ── Primary Payment Gateway (e.g., Stripe) ──────────────────────────────

/// Health check for the primary payment gateway.
/// In production, this would verify API key validity, check rate limits,
/// and confirm the gateway's status page is green.
async fn primary_health() -> VilResponse<PaymentResult> {
    VilResponse::ok(PaymentResult {
        gateway: "stripe".into(),
        role: "Primary payment gateway — handles all traffic by default".into(),
        charge_id: "n/a".into(),
        amount_cents: 0,
        currency: "USD".into(),
        status: "healthy",
        retry_strategy: "Retry(3) with exponential backoff before failover to backup".into(),
    })
}

/// Primary gateway charge endpoint.
/// If this fails, VIL automatically retries 3 times, then fails over to backup.
async fn primary_charge(body: ShmSlice) -> Result<VilResponse<PaymentResult>, VilError> {
    let req: PaymentRequest = body.json()
        .map_err(|_| VilError::bad_request("Invalid payment JSON — need card_token, amount_cents, currency"))?;

    // Simulate charge processing (in production: call Stripe API)
    let charge_id = format!("ch_stripe_{}", req.amount_cents);

    Ok(VilResponse::ok(PaymentResult {
        gateway: "stripe".into(),
        role: "Primary — processed successfully".into(),
        charge_id,
        amount_cents: req.amount_cents,
        currency: req.currency,
        status: "charged",
        retry_strategy: "Did not need retry — primary succeeded on first attempt".into(),
    }))
}

// ── Backup Payment Gateway (e.g., Adyen) ────────────────────────────────

/// Health check for the backup payment gateway.
/// The backup sits in hot standby — it receives no traffic unless primary fails.
async fn backup_health() -> VilResponse<PaymentResult> {
    VilResponse::ok(PaymentResult {
        gateway: "adyen".into(),
        role: "Hot standby — activates only after primary exhausts all retries".into(),
        charge_id: "n/a".into(),
        amount_cents: 0,
        currency: "USD".into(),
        status: "standby",
        retry_strategy: "Immediate takeover — no additional retries on backup".into(),
    })
}

/// Backup gateway charge endpoint.
/// Only called after primary fails 3 retries. Processes the same payment.
async fn backup_charge(body: ShmSlice) -> Result<VilResponse<PaymentResult>, VilError> {
    let req: PaymentRequest = body.json()
        .map_err(|_| VilError::bad_request("Invalid payment JSON"))?;

    let charge_id = format!("ch_adyen_{}", req.amount_cents);

    Ok(VilResponse::ok(PaymentResult {
        gateway: "adyen".into(),
        role: "Backup — activated after primary failed 3 retries".into(),
        charge_id,
        amount_cents: req.amount_cents,
        currency: req.currency,
        status: "charged_via_failover",
        retry_strategy: "Failover completed — customer saw no interruption".into(),
    }))
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  032 — Payment Gateway HA (Failover + Retry Strategy)               ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Primary: Stripe → retry 3 times on failure                          ║");
    println!("║  Backup:  Adyen  → activates after primary exhausts retries          ║");
    println!("║  Customer sees zero interruption during gateway outages              ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // Primary gateway: Stripe — handles all payment traffic by default
    let primary_gateway = ServiceProcess::new("primary")
        .endpoint(Method::GET, "/health", get(primary_health))
        .endpoint(Method::POST, "/charge", post(primary_charge));

    // Backup gateway: Adyen — hot standby, activates on primary failure
    let backup_gateway = ServiceProcess::new("backup")
        .endpoint(Method::GET, "/health", get(backup_health))
        .endpoint(Method::POST, "/charge", post(backup_charge));

    // Declarative failover: one line replaces hundreds of lines of
    // manual retry logic, circuit breakers, and fallback handlers.
    // FailoverStrategy::Retry(3) means: try primary 3 times, then switch to backup.
    VilApp::new("payment-gateway-ha")
        .port(8080)
        .service(primary_gateway)
        .service(backup_gateway)
        .failover(VxFailoverConfig::new()
            .backup("primary", "backup", FailoverStrategy::Retry(3)))
        .run()
        .await;
}
