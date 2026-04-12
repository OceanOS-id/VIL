use serde_json::Value;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};

pub fn jwt_sign(args: &[Value]) -> Result<Value, String> {
    let payload = args.get(0).ok_or("jwt_sign: payload required")?;
    let secret = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("jwt_sign: secret required")?;
    let algo = args.get(2).and_then(|v| v.as_str()).unwrap_or("HS256");
    let algorithm = match algo {
        "HS384" => Algorithm::HS384,
        "HS512" => Algorithm::HS512,
        _ => Algorithm::HS256,
    };
    let header = Header::new(algorithm);
    let token = encode(
        &header,
        payload,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("jwt_sign: {}", e))?;
    Ok(Value::String(token))
}

pub fn jwt_verify(args: &[Value]) -> Result<Value, String> {
    let token = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("jwt_verify: token required")?;
    let secret = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("jwt_verify: secret required")?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    validation.validate_exp = false;
    let data = decode::<Value>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| format!("jwt_verify: {}", e))?;
    Ok(data.claims)
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("jwt_sign", jwt_sign), ("jwt_verify", jwt_verify)]
}
