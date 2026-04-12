use serde_json::Value;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

pub fn sha256(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).and_then(|v| v.as_str()).ok_or("sha256: string arg required")?;
    let hash = Sha256::digest(data.as_bytes());
    Ok(Value::String(hex::encode(hash)))
}

pub fn md5(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).and_then(|v| v.as_str()).ok_or("md5: string arg required")?;
    let hash = md5::Md5::digest(data.as_bytes());
    Ok(Value::String(hex::encode(hash)))
}

pub fn hmac_sha256(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).and_then(|v| v.as_str()).ok_or("hmac_sha256: data arg required")?;
    let key = args.get(1).and_then(|v| v.as_str()).ok_or("hmac_sha256: key arg required")?;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(|e| format!("hmac: {}", e))?;
    mac.update(data.as_bytes());
    Ok(Value::String(hex::encode(mac.finalize().into_bytes())))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![
        ("sha256", sha256),
        ("md5", md5),
        ("hmac_sha256", hmac_sha256),
    ]
}
