//! Currency Exchange — Rust WASM Module
//! Handles: rates (GET), convert (POST), stats (GET)
//! Compile: rustc --target wasm32-wasi currency_convert.rs -o currency_convert.wasm

use std::io::{self, Read};

static RATES: &[(& str, &str, f64, f64)] = &[
    ("USD", "US Dollar",          15850.0, 1.0),
    ("EUR", "Euro",               17200.0, 1.2),
    ("SGD", "Singapore Dollar",   11800.0, 1.5),
    ("MYR", "Malaysian Ringgit",   3560.0, 2.0),
    ("JPY", "Japanese Yen",         105.0, 1.8),
    ("AUD", "Australian Dollar",  10300.0, 1.5),
    ("GBP", "British Pound",      20100.0, 1.0),
    ("CNY", "Chinese Yuan",        2180.0, 2.5),
    ("THB", "Thai Baht",            460.0, 2.0),
    ("SAR", "Saudi Riyal",         4225.0, 1.5),
];

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let operation = extract_str(&input, "operation").unwrap_or("convert".into());

    match operation.as_str() {
        "rates" => handle_rates(),
        "convert" => handle_convert(&input),
        "stats" => handle_stats(&input),
        _ => handle_convert(&input),
    }
}

fn handle_rates() {
    let mut rates_json = String::from("[");
    for (i, (code, name, mid, spread)) in RATES.iter().enumerate() {
        let half = spread / 200.0;
        let buy = mid * (1.0 - half);
        let sell = mid * (1.0 + half);
        if i > 0 { rates_json.push(','); }
        rates_json.push_str(&format!(
            r#"{{"code":"{}","name":"{}","buy_rate":{:.2},"sell_rate":{:.2},"mid_rate":{},"spread_pct":{}}}"#,
            code, name, buy, sell, mid, spread
        ));
    }
    rates_json.push(']');

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    println!(r#"{{"base":"IDR","rates":{},"updated_at":{}}}"#, rates_json, now);
}

fn handle_convert(input: &str) {
    let amount = extract_f64(input, "amount").unwrap_or(0.0);
    let from = extract_str(input, "from").unwrap_or("USD".into());
    let to = extract_str(input, "to").unwrap_or("IDR".into());

    // Validation
    if amount <= 0.0 {
        println!(r#"{{"_status":400,"error":"amount must be positive"}}"#);
        return;
    }

    let from_rate = find_rate(&from);
    let to_rate = find_rate(&to);

    if !from.eq_ignore_ascii_case("IDR") && from_rate.is_none() {
        println!(r#"{{"_status":404,"error":"currency {} not supported"}}"#, from);
        return;
    }
    if !to.eq_ignore_ascii_case("IDR") && to_rate.is_none() {
        println!(r#"{{"_status":404,"error":"currency {} not supported"}}"#, to);
        return;
    }

    // Convert: from → IDR → to
    let (idr_amount, from_spread) = if from.eq_ignore_ascii_case("IDR") {
        (amount, 0.0)
    } else {
        let (_, _, mid, spread) = from_rate.unwrap();
        let buy = mid * (1.0 - spread / 200.0);
        (amount * buy, *spread)
    };

    let (converted, to_spread) = if to.eq_ignore_ascii_case("IDR") {
        (idr_amount, 0.0)
    } else {
        let (_, _, mid, spread) = to_rate.unwrap();
        let sell = mid * (1.0 + spread / 200.0);
        (idr_amount / sell, *spread)
    };

    let rate_applied = if amount > 0.0 { converted / amount } else { 0.0 };
    let converted_amount = (converted * 100.0).round() / 100.0;
    let spread_pct = if from_spread > to_spread { from_spread } else { to_spread };

    // Read conversion_id from counter (passed via input)
    let conversion_id = extract_f64(input, "conversion_id").unwrap_or(1.0) as u64;

    println!(r#"{{"from":"{}","to":"{}","amount":{},"rate_applied":{},"converted_amount":{},"spread_pct":{},"conversion_id":{}}}"#,
        from.to_uppercase(), to.to_uppercase(), amount, rate_applied, converted_amount, spread_pct, conversion_id);
}

fn handle_stats(input: &str) {
    let total = extract_f64(input, "total_conversions").unwrap_or(0.0) as u64;
    let volume = extract_f64(input, "total_volume_idr").unwrap_or(0.0) as u64;
    let uptime = extract_f64(input, "uptime_secs").unwrap_or(0.0) as u64;
    println!(r#"{{"total_conversions":{},"total_volume_idr":{},"uptime_secs":{}}}"#, total, volume, uptime);
}

fn find_rate(code: &str) -> Option<&'static (&'static str, &'static str, f64, f64)> {
    RATES.iter().find(|(c, _, _, _)| c.eq_ignore_ascii_case(code))
}

fn extract_f64(json: &str, key: &str) -> Option<f64> {
    let pattern = format!(r#""{}":"#, key);
    json.find(&pattern).and_then(|pos| {
        let start = pos + pattern.len();
        let rest = json[start..].trim();
        let end = rest.find(|c: char| !c.is_numeric() && c != '.' && c != '-').unwrap_or(rest.len());
        rest[..end].parse().ok()
    })
}

fn extract_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!(r#""{}":""#, key);
    json.find(&pattern).and_then(|pos| {
        let start = pos + pattern.len();
        let end = json[start..].find('"').unwrap_or(0);
        Some(json[start..start + end].to_string())
    })
}
