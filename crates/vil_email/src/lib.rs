use serde_json::{json, Value};
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;

pub fn send_email(args: &[Value]) -> Result<Value, String> {
    let to = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("send_email: 'to' required")?;
    let subject = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("send_email: 'subject' required")?;
    let body = args
        .get(2)
        .and_then(|v| v.as_str())
        .ok_or("send_email: 'body' required")?;

    let smtp_host =
        std::env::var("VIL_SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
    let smtp_port = std::env::var("VIL_SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(587u16);
    let smtp_user = std::env::var("VIL_SMTP_USER").unwrap_or_default();
    let smtp_pass = std::env::var("VIL_SMTP_PASS").unwrap_or_default();
    let from = std::env::var("VIL_SMTP_FROM")
        .unwrap_or_else(|_| format!("noreply@{}", smtp_host));

    let email = Message::builder()
        .from(
            from.parse()
                .map_err(|e| format!("send_email: invalid from: {}", e))?,
        )
        .to(to
            .parse()
            .map_err(|e| format!("send_email: invalid to: {}", e))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("send_email: {}", e))?;

    let transport = if smtp_user.is_empty() {
        SmtpTransport::builder_dangerous(&smtp_host)
            .port(smtp_port)
            .build()
    } else {
        SmtpTransport::relay(&smtp_host)
            .map_err(|e| format!("send_email: {}", e))?
            .port(smtp_port)
            .credentials(Credentials::new(smtp_user, smtp_pass))
            .build()
    };

    transport
        .send(&email)
        .map_err(|e| format!("send_email: {}", e))?;
    Ok(json!({"sent": true, "to": to, "subject": subject}))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("send_email", send_email)]
}
