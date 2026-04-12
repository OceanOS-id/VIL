use serde_json::{json, Value};

pub fn parse_phone(args: &[Value]) -> Result<Value, String> {
    let number = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("parse_phone: number required")?;
    let _country = args.get(1).and_then(|v| v.as_str()).unwrap_or("ID");

    let country_id = phonenumber::country::ID;

    match phonenumber::parse(Some(country_id), number) {
        Ok(phone) => {
            let valid = phonenumber::is_valid(&phone);
            let formatted = phonenumber::format(&phone)
                .mode(phonenumber::Mode::E164)
                .to_string();
            let national = phonenumber::format(&phone)
                .mode(phonenumber::Mode::National)
                .to_string();
            Ok(json!({
                "valid": valid,
                "e164": formatted,
                "national": national,
                "country_code": phone.code().value(),
                "national_number": phone.national().to_string(),
            }))
        }
        Err(e) => Ok(json!({
            "valid": false,
            "error": e.to_string(),
            "input": number
        })),
    }
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("parse_phone", parse_phone)]
}
