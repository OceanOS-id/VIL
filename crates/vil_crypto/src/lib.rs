use serde_json::Value;
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rand::RngCore;

pub fn aes_encrypt(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).and_then(|v| v.as_str()).ok_or("aes_encrypt: data required")?;
    let key_str = args.get(1).and_then(|v| v.as_str()).ok_or("aes_encrypt: key required (32 bytes hex)")?;
    let key_bytes = hex::decode(key_str).map_err(|e| format!("aes_encrypt: invalid hex key: {}", e))?;
    if key_bytes.len() != 32 {
        return Err("aes_encrypt: key must be 32 bytes (64 hex chars)".into());
    }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("aes_encrypt: {}", e))?;
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(Value::String(B64.encode(&result)))
}

pub fn aes_decrypt(args: &[Value]) -> Result<Value, String> {
    let encrypted = args.get(0).and_then(|v| v.as_str()).ok_or("aes_decrypt: data required")?;
    let key_str = args.get(1).and_then(|v| v.as_str()).ok_or("aes_decrypt: key required")?;
    let key_bytes = hex::decode(key_str).map_err(|e| format!("aes_decrypt: invalid hex key: {}", e))?;
    if key_bytes.len() != 32 {
        return Err("aes_decrypt: key must be 32 bytes (64 hex chars)".into());
    }
    let raw = B64.decode(encrypted).map_err(|e| format!("aes_decrypt: invalid base64: {}", e))?;
    if raw.len() < 12 {
        return Err("aes_decrypt: data too short".into());
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("aes_decrypt: {}", e))?;
    Ok(Value::String(String::from_utf8_lossy(&plaintext).into_owned()))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("aes_encrypt", aes_encrypt), ("aes_decrypt", aes_decrypt)]
}
