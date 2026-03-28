// ╔════════════════════════════════════════════════════════════════════════╗
// ║  031 — Banking Transaction Mesh (Multi-Service Mesh Routing)        ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: VxMeshConfig, MeshRouteEntry, Lane variants               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A bank teller submits a transaction. The system routes    ║
// ║  it through three services in a mesh topology:                       ║
// ║                                                                      ║
// ║    teller → fraud_check (Data Lane)                                  ║
// ║      "Send transaction details for fraud analysis"                   ║
// ║    fraud_check → core_banking (Data Lane)                            ║
// ║      "Forward approved transaction for ledger posting"               ║
// ║    core_banking → notification (Control Lane)                        ║
// ║      "Notify customer of completed transaction"                      ║
// ║                                                                      ║
// ║  Why mesh routing matters in banking:                                ║
// ║    - Each service is independently deployable and scalable           ║
// ║    - Fraud check can be swapped without touching teller or banking   ║
// ║    - Control Lane for notifications ensures customer gets notified   ║
// ║      even if the Data Lane is saturated with batch transactions      ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-mesh-routing
// Test: curl http://localhost:8080/api/teller/ping
//       curl -X POST http://localhost:8080/api/teller/submit \
//         -H 'Content-Type: application/json' \
//         -d '{"from_account":"ACC-1001","to_account":"ACC-2002","amount_cents":50000,"currency":"USD"}'

use vil_server::prelude::*;

// ── Business Domain Types ───────────────────────────────────────────────

/// Transaction submitted by a bank teller at the counter.
/// In production, this would include branch_id, teller_id, auth tokens, etc.
#[derive(Deserialize)]
struct TransactionRequest {
    from_account: String,
    to_account: String,
    amount_cents: u64,
    currency: String,
}

/// Result from the fraud detection service.
/// The fraud engine scores each transaction from 0 (safe) to 100 (fraud).
#[derive(Serialize)]
struct FraudCheckResult {
    service: &'static str,
    transaction_ref: String,
    fraud_score: u8,
    is_approved: bool,
    rule_triggered: &'static str,
}

/// Result from core banking after posting to the ledger.
#[derive(Serialize)]
struct TransactionResult {
    service: &'static str,
    status: &'static str,
    ledger_entry_id: u64,
    from_account: String,
    to_account: String,
    amount_cents: u64,
}

/// Notification service acknowledgment.
#[derive(Serialize)]
struct NotificationAck {
    service: &'static str,
    channel: &'static str,
    customer_notified: bool,
}

// ── Teller Service ──────────────────────────────────────────────────────

/// Teller submits a transaction. In production, this is the branch counter
/// application where bank employees enter customer requests.
async fn teller_submit(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<FraudCheckResult>, VilError> {
    let txn: TransactionRequest = body
        .json()
        .map_err(|_| VilError::bad_request("Invalid transaction JSON"))?;

    // Generate a reference number for tracking
    let txn_ref = format!("TXN-{}-{}", txn.from_account, txn.amount_cents);

    // Forward transaction to fraud detection via Data Lane.
    // The full transaction payload travels zero-copy through SHM.
    ctx.send("fraud_check", body.as_bytes()).await?;

    // Simple fraud scoring (in production: ML model + rule engine)
    let fraud_score = if txn.amount_cents > 1_000_000 { 75 } else { 12 };
    let is_approved = fraud_score < 50;

    Ok(VilResponse::ok(FraudCheckResult {
        service: "fraud_check",
        transaction_ref: txn_ref,
        fraud_score,
        is_approved,
        rule_triggered: if is_approved {
            "none"
        } else {
            "high_value_transfer"
        },
    }))
}

/// Teller service health/info endpoint.
async fn teller_ping() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "service": "teller",
        "role": "Bank teller counter — submits customer transactions",
        "mesh_routes": "teller → fraud_check (Data), fraud_check → core_banking (Data), core_banking → notification (Control)"
    }))
}

// ── Fraud Check Service ─────────────────────────────────────────────────

/// Fraud check processes the transaction and forwards approved ones
/// to core banking. In production: calls ML scoring API, checks
/// blacklists, velocity rules, geo-anomaly detection.
async fn fraud_process(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<FraudCheckResult>, VilError> {
    let data = body.as_bytes();
    // After fraud analysis passes, forward to core banking for ledger posting
    ctx.send("core_banking", data).await?;
    Ok(VilResponse::ok(FraudCheckResult {
        service: "fraud_check",
        transaction_ref: "analyzed".into(),
        fraud_score: 8,
        is_approved: true,
        rule_triggered: "none",
    }))
}

// ── Core Banking Service ────────────────────────────────────────────────

/// Core banking posts the transaction to the ledger and notifies customer.
/// Uses Control Lane for notification — ensures delivery even under load.
async fn core_banking_post(ctx: ServiceCtx) -> Result<VilResponse<TransactionResult>, VilError> {
    // After posting to ledger, send notification via Control Lane.
    // Control Lane is chosen because customer notifications are critical
    // and must not be delayed by bulk data transfers (e.g., end-of-day batch).
    ctx.control("notification", b"txn_completed").await?;

    Ok(VilResponse::ok(TransactionResult {
        service: "core_banking",
        status: "posted_to_ledger",
        ledger_entry_id: 98765,
        from_account: "ACC-1001".into(),
        to_account: "ACC-2002".into(),
        amount_cents: 50000,
    }))
}

// ── Notification Service ────────────────────────────────────────────────

/// Notification service sends SMS/email/push to the customer.
/// Receives commands via Control Lane (priority channel).
async fn notification_send() -> VilResponse<NotificationAck> {
    VilResponse::ok(NotificationAck {
        service: "notification",
        channel: "sms+push",
        customer_notified: true,
    })
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  031 — Banking Transaction Mesh (Multi-Service Routing)              ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  teller → fraud_check (Data) → core_banking (Data)                   ║");
    println!("║  core_banking → notification (Control — priority channel)             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // Teller: public-facing, where bank employees submit transactions
    let teller_svc = ServiceProcess::new("teller")
        .endpoint(Method::GET, "/ping", get(teller_ping))
        .endpoint(Method::POST, "/submit", post(teller_submit));

    // Fraud Check: internal service, analyzes transactions for suspicious patterns
    let fraud_svc = ServiceProcess::new("fraud_check")
        .visibility(Visibility::Internal)
        .endpoint(Method::POST, "/analyze", post(fraud_process));

    // Core Banking: internal service, posts approved transactions to the ledger
    let banking_svc = ServiceProcess::new("core_banking")
        .visibility(Visibility::Internal)
        .endpoint(Method::POST, "/post", post(core_banking_post));

    // Notification: internal service, sends customer alerts via SMS/push/email
    let notification_svc = ServiceProcess::new("notification")
        .visibility(Visibility::Internal)
        .endpoint(Method::GET, "/send", get(notification_send));

    // Wire the banking mesh: three Data hops + one Control hop
    VilApp::new("banking-transaction-mesh")
        .port(8080)
        .service(teller_svc)
        .service(fraud_svc)
        .service(banking_svc)
        .service(notification_svc)
        .mesh(
            VxMeshConfig::new()
                .route("teller", "fraud_check", VxLane::Data) // transaction details
                .route("fraud_check", "core_banking", VxLane::Data) // approved transaction
                .route("core_banking", "notification", VxLane::Control),
        ) // customer alert (priority)
        .run()
        .await;
}
