// 030 — Tri-Lane Messaging (Order Pipeline)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/030-basic-trilane-messaging/vwfd/workflows", 8080)
        .native("trilane_order_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let customer_id = body.get("customer_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let product_id = body.get("product_id").and_then(|v| v.as_u64()).unwrap_or(0);
            let quantity = body.get("quantity").and_then(|v| v.as_u64()).unwrap_or(1);
            let amount_cents = body.get("amount_cents").and_then(|v| v.as_u64()).unwrap_or(0);
            Ok(json!({
                "order_id": format!("ORD-{}-{}", customer_id, product_id),
                "customer_id": customer_id,
                "product_id": product_id,
                "quantity": quantity,
                "amount_cents": amount_cents,
                "fulfillment_notified": true,
                "inventory_reserved": true,
                "lanes": {"trigger": "fired", "data": "routed", "control": "ack"}
            }))
        })
        .run()
        .await;
}
