use serde_json::{json, Value};
use chrono::{NaiveDateTime, NaiveDate, DateTime, Utc, Datelike, Timelike};

pub fn parse_date(args: &[Value]) -> Result<Value, String> {
    let s = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("parse_date: string required")?;
    let fmt = args.get(1).and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d");

    if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
        return Ok(json!({
            "year": dt.year(),
            "month": dt.month(),
            "day": dt.day(),
            "hour": dt.hour(),
            "minute": dt.minute(),
            "second": dt.second(),
            "iso": dt.format("%Y-%m-%dT%H:%M:%S").to_string()
        }));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
        return Ok(json!({
            "year": d.year(),
            "month": d.month(),
            "day": d.day(),
            "iso": d.format("%Y-%m-%d").to_string()
        }));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(json!({
            "year": dt.year(),
            "month": dt.month(),
            "day": dt.day(),
            "hour": dt.hour(),
            "minute": dt.minute(),
            "second": dt.second(),
            "iso": dt.to_rfc3339()
        }));
    }
    Err(format!(
        "parse_date: cannot parse '{}' with format '{}'",
        s, fmt
    ))
}

pub fn format_date(args: &[Value]) -> Result<Value, String> {
    let s = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("format_date: date string required")?;
    let fmt = args.get(1).and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d");

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(Value::String(dt.format(fmt).to_string()));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Value::String(dt.format(fmt).to_string()));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(Value::String(d.format(fmt).to_string()));
    }
    Err(format!("format_date: cannot parse '{}'", s))
}

pub fn now(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(Utc::now().to_rfc3339()))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![
        ("parse_date", parse_date),
        ("format_date", format_date),
        ("now", now),
    ]
}
