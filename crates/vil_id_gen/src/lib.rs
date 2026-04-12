use serde_json::Value;

pub fn uuid_v4(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(uuid::Uuid::new_v4().to_string()))
}

pub fn uuid_v7(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(uuid::Uuid::now_v7().to_string()))
}

pub fn ulid(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(ulid::Ulid::new().to_string()))
}

pub fn nanoid(args: &[Value]) -> Result<Value, String> {
    let len = args.get(0).and_then(|v| v.as_u64()).unwrap_or(21) as usize;
    let alphabet: Vec<char> =
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            .chars()
            .collect();
    let id: String = (0..len)
        .map(|_| {
            let idx = (rand::random::<u8>() as usize) % alphabet.len();
            alphabet[idx]
        })
        .collect();
    Ok(Value::String(id))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![
        ("uuid_v4", uuid_v4),
        ("uuid_v7", uuid_v7),
        ("ulid", ulid),
        ("nanoid", nanoid),
    ]
}
