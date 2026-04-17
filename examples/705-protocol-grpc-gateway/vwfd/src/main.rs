// 705 — Payment Gateway (VWFD)
// Business logic identical to standard:
//   gRPC trigger: receives charge stream from PaymentService/StreamCharges
//   Webhook endpoints: GET /:id (lookup), POST /refund (refund)
// Response fields match standard proto: ChargeResponse, PaymentRecord, RefundResponse
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static PAY_SEQ: AtomicU64 = AtomicU64::new(1);

fn store_charge(input: &Value) -> Result<Value, String> {
    let charge = &input["charge"];
    let customer_id = input["customer_id"].as_str().unwrap_or("unknown");
    let payment_id = charge["payment_id"].as_str().unwrap_or("PAY-00000");
    let status = charge["status"].as_str().unwrap_or("approved");
    Ok(json!({
        "stored": true,
        "payment_id": payment_id,
        "customer_id": customer_id,
        "status": status
    }))
}

fn get_payment(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let id = path.split('/').last().unwrap_or("PAY-00001");
    Ok(json!({
        "payment_id": id,
        "customer_id": "C-001",
        "amount_cents": 5000,
        "currency": "USD",
        "status": "approved",
        "description": "Order #1234",
        "created_at": 1705312800_u64
    }))
}

fn process_refund(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    let payment_id = body["payment_id"].as_str().unwrap_or("PAY-00001");
    let seq = PAY_SEQ.fetch_add(1, Ordering::Relaxed);
    let refund_id = format!("REF-{:05}", seq);
    Ok(json!({
        "refund_id": refund_id,
        "payment_id": payment_id,
        "status": "refunded",
        "reason": body["reason"].as_str().unwrap_or("customer_request")
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/705-protocol-grpc-gateway/vwfd/workflows", 3705)
        .wasm("payment_validate_card", "examples/705-protocol-grpc-gateway/vwfd/wasm/java/PaymentProcessor.class")
        .wasm("payment_process_charge", "examples/705-protocol-grpc-gateway/vwfd/wasm/java/PaymentProcessor.class")
        .native("store_charge", store_charge)
        .native("get_payment", get_payment)
        .native("process_refund", process_refund)
        .run().await;
}
