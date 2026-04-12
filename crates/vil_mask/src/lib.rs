use serde_json::Value;
use regex::Regex;

pub fn mask_pii(args: &[Value]) -> Result<Value, String> {
    let data = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("mask_pii: string required")?;
    let rule = args.get(1).and_then(|v| v.as_str()).unwrap_or("auto");
    let masked = match rule {
        "email" => {
            let re =
                Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
            re.replace_all(data, "***@***.***").to_string()
        }
        "phone" => {
            let re = Regex::new(r"\d{4,}").unwrap();
            re.replace_all(data, |caps: &regex::Captures| {
                let s = caps.get(0).unwrap().as_str();
                if s.len() > 4 {
                    format!("{}****", &s[..s.len() - 4])
                } else {
                    "****".to_string()
                }
            })
            .to_string()
        }
        "nik" => {
            if data.len() == 16 {
                format!("{}********{}", &data[..4], &data[12..])
            } else {
                "****************".to_string()
            }
        }
        "cc" | "credit_card" => {
            if data.len() >= 12 {
                format!(
                    "{}********{}",
                    &data[..4],
                    &data[data.len() - 4..]
                )
            } else {
                "****".to_string()
            }
        }
        _ => {
            // auto: mask middle portion
            let len = data.len();
            if len <= 4 {
                "****".to_string()
            } else {
                format!("{}{}{}", &data[..2], "*".repeat(len - 4), &data[len - 2..])
            }
        }
    };
    Ok(Value::String(masked))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("mask_pii", mask_pii)]
}
