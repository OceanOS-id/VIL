use serde_json::{json, Value};

pub fn send_webhook(args: &[Value]) -> Result<Value, String> {
    let url = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("send_webhook: url required")?;
    let payload = args.get(1).ok_or("send_webhook: payload required")?;
    let secret = args.get(2).and_then(|v| v.as_str());

    let body = serde_json::to_string(payload).unwrap_or_default();

    let mut req = ureq::post(url).set("Content-Type", "application/json");

    // HMAC signature if secret provided
    if let Some(secret) = secret {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| format!("send_webhook: hmac: {}", e))?;
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        req = req.set(
            "X-Webhook-Signature",
            &format!("sha256={}", signature),
        );
    }

    let resp = req
        .send_string(&body)
        .map_err(|e| format!("send_webhook: {}", e))?;
    Ok(json!({"sent": true, "status": resp.status(), "url": url}))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("send_webhook", send_webhook)]
}
