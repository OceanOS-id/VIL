use serde_json::{json, Value};
use chrono::{NaiveDate, Utc, Datelike};

pub fn age(args: &[Value]) -> Result<Value, String> {
    let birth = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("age: birth_date string required")?;
    let birth_date = NaiveDate::parse_from_str(birth, "%Y-%m-%d")
        .map_err(|e| format!("age: invalid date '{}': {}", birth, e))?;
    let today = Utc::now().date_naive();
    let mut years = today.year() - birth_date.year();
    if today.ordinal() < birth_date.ordinal() {
        years -= 1;
    }
    Ok(json!(years))
}

pub fn duration(args: &[Value]) -> Result<Value, String> {
    let from = args
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or("duration: from date required")?;
    let to = args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or("duration: to date required")?;
    let unit = args.get(2).and_then(|v| v.as_str()).unwrap_or("days");
    let from_date =
        NaiveDate::parse_from_str(from, "%Y-%m-%d").map_err(|e| format!("duration: {}", e))?;
    let to_date =
        NaiveDate::parse_from_str(to, "%Y-%m-%d").map_err(|e| format!("duration: {}", e))?;
    let days = (to_date - from_date).num_days();
    let result = match unit {
        "days" => days as f64,
        "weeks" => days as f64 / 7.0,
        "months" => days as f64 / 30.44,
        "years" => days as f64 / 365.25,
        _ => days as f64,
    };
    Ok(json!(result))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("age", age), ("duration", duration)]
}
