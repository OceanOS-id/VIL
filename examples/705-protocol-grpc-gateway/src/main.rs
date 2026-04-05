// ╔════════════════════════════════════════════════════════════╗
// ║  705 — Payment Processing Gateway (gRPC-style)            ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   FinTech — Payment Processing                    ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: ServiceCtx, ShmSlice, VilResponse, VilModel,   ║
// ║            typed request/response contracts (gRPC-style)   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Payment gateway with charge, status, refund.    ║
// ║  Uses VIL typed contracts (proto-equivalent) — ready for   ║
// ║  dual HTTP+gRPC serving when vil_grpc is fully wired.      ║
// ║                                                             ║
// ║  Note: vil_grpc wraps tonic with GrpcGatewayBuilder but    ║
// ║  requires proto-generated service impls. This example uses  ║
// ║  VilApp HTTP with gRPC-style typed contracts.              ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-protocol-grpc-gateway
// Test:
//   curl -X POST http://localhost:3705/api/payments/charge \
//     -H 'Content-Type: application/json' \
//     -d '{"customer_id":"C-001","amount_cents":5000,"currency":"USD","description":"Order #1234"}'
//   curl http://localhost:3705/api/payments/PAY-00001
//   curl -X POST http://localhost:3705/api/payments/refund \
//     -H 'Content-Type: application/json' \
//     -d '{"payment_id":"PAY-00001","reason":"customer request"}'

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;

// ── Models (gRPC-style typed contracts) ──────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChargeRequest {
    customer_id: String,
    amount_cents: i64,
    currency: String,
    #[serde(default)]
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ChargeResponse {
    payment_id: String,
    status: String,
    amount_cents: i64,
    currency: String,
    customer_id: String,
    created_at: u64,
}

#[derive(Debug, Deserialize)]
struct RefundRequest {
    payment_id: String,
    #[serde(default)]
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RefundResponse {
    refund_id: String,
    payment_id: String,
    status: String,
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PaymentRecord {
    payment_id: String,
    customer_id: String,
    amount_cents: i64,
    currency: String,
    status: String,
    description: String,
    created_at: u64,
}

struct PaymentState {
    payments: Mutex<HashMap<String, PaymentRecord>>,
    next_id: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

async fn charge(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<ChargeResponse>> {
    let req: ChargeRequest = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.amount_cents <= 0 {
        return Err(VilError::bad_request("amount_cents must be positive"));
    }
    if req.customer_id.is_empty() {
        return Err(VilError::bad_request("customer_id required"));
    }

    let state = ctx.state::<Arc<PaymentState>>().map_err(|_| VilError::internal("state"))?;
    let id_num = state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let payment_id = format!("PAY-{:05}", id_num);

    // Deterministic approval: decline > $10,000
    let status = if req.amount_cents > 1_000_000 { "declined" } else { "approved" };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    let record = PaymentRecord {
        payment_id: payment_id.clone(),
        customer_id: req.customer_id.clone(),
        amount_cents: req.amount_cents,
        currency: req.currency.clone(),
        status: status.into(),
        description: req.description,
        created_at: now,
    };

    state.payments.lock().unwrap().insert(payment_id.clone(), record);

    Ok(VilResponse::ok(ChargeResponse {
        payment_id,
        status: status.into(),
        amount_cents: req.amount_cents,
        currency: req.currency,
        customer_id: req.customer_id,
        created_at: now,
    }))
}

async fn get_payment(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<PaymentRecord>> {
    let state = ctx.state::<Arc<PaymentState>>().map_err(|_| VilError::internal("state"))?;
    let payments = state.payments.lock().unwrap();
    let record = payments.get(&id)
        .ok_or_else(|| VilError::not_found(format!("payment {} not found", id)))?;
    Ok(VilResponse::ok(record.clone()))
}

async fn refund(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<RefundResponse>> {
    let req: RefundRequest = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let state = ctx.state::<Arc<PaymentState>>().map_err(|_| VilError::internal("state"))?;

    let mut payments = state.payments.lock().unwrap();
    let record = payments.get_mut(&req.payment_id)
        .ok_or_else(|| VilError::not_found(format!("payment {} not found", req.payment_id)))?;

    if record.status == "refunded" {
        return Err(VilError::bad_request("already refunded"));
    }
    if record.status != "approved" {
        return Err(VilError::bad_request(format!("cannot refund {} payment", record.status)));
    }

    record.status = "refunded".into();
    let refund_id = format!("REF-{}", &req.payment_id[4..]);

    Ok(VilResponse::ok(RefundResponse {
        refund_id,
        payment_id: req.payment_id,
        status: "refunded".into(),
        reason: req.reason,
    }))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(PaymentState {
        payments: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("payments")
        .endpoint(Method::POST, "/charge", post(charge))
        .endpoint(Method::GET, "/:id", get(get_payment))
        .endpoint(Method::POST, "/refund", post(refund))
        .state(state);

    VilApp::new("payment-gateway")
        .port(3705)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
