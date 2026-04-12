// 905 — Notification Hub (Standard Pattern)
// Demonstrates: vil_email, vil_webhook_out, vil_template, vil_mask, vil_id_gen
use serde_json::{json, Value};

fn main() {
    // 1. Generate notification ID
    let notif_id = vil_id_gen::ulid(&[]).unwrap();
    println!("Notification ID: {}", notif_id);

    // 2. Render email body from template
    let body = vil_template::render_template(&[
        Value::String("Dear {{name}},\n\nYour order #{{order_id}} has been confirmed.\nTotal: Rp {{total}}\n\nThank you!".into()),
        json!({"name": "Alice", "order_id": "ORD-2024-001", "total": "1,500,000"}),
    ]).unwrap();
    println!("Email body:\n{}", body);

    // 3. Mask PII before logging
    let masked_email = vil_mask::mask_pii(&[Value::String("alice@example.com".into()), Value::String("email".into())]).unwrap();
    let masked_phone = vil_mask::mask_pii(&[Value::String("081234567890".into()), Value::String("phone".into())]).unwrap();
    println!("Masked: email={}, phone={}", masked_email, masked_phone);

    // 4. Send email (requires VIL_SMTP_HOST env)
    // Uncomment when SMTP is configured:
    // let sent = vil_email::send_email(&[
    //     Value::String("alice@example.com".into()),
    //     Value::String("Order Confirmed".into()),
    //     body.clone(),
    // ]).unwrap();
    // println!("Email sent: {}", sent);
    println!("Email: skipped (set VIL_SMTP_HOST to enable)");

    // 5. Send webhook notification (requires external URL)
    // Uncomment when webhook URL is configured:
    // let webhook = vil_webhook_out::send_webhook(&[
    //     Value::String("https://hooks.example.com/notify".into()),
    //     json!({"event": "order.confirmed", "order_id": "ORD-2024-001"}),
    //     Value::String("webhook-secret".into()),
    // ]).unwrap();
    // println!("Webhook sent: {}", webhook);
    println!("Webhook: skipped (requires external URL)");
}
