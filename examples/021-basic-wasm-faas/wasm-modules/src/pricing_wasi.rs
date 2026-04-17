// pricing_wasi.wasm — Pricing rules via WASI stdin/stdout
// Protocol: host sends JSON to stdin, WASM writes JSON result to stdout
// Build: rustc --target wasm32-wasip1 --edition 2021 -O pricing_wasi.rs -o ../out/pricing_wasi.wasm

fn main() {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).unwrap_or_default();

    let func = extract_str(&input, "function").unwrap_or_default();
    let args = extract_array_i32(&input, "args");

    let result = match func.as_str() {
        "calculate_price" => {
            let base = args.get(0).copied().unwrap_or(0);
            let qty = args.get(1).copied().unwrap_or(1);
            let subtotal = base * qty;
            let discount_pct = match qty {
                0..=4 => 0,
                5..=9 => 5,
                10..=49 => 10,
                _ => 20,
            };
            let discount = subtotal * discount_pct / 100;
            let total = subtotal - discount;
            let tax = total * 11 / 100;
            format!("{{\"total\":{},\"discount\":{},\"tax\":{},\"tier\":\"{}\"}}",
                total + tax, discount, tax,
                match qty { 0..=4 => "RETAIL", 5..=9 => "SMALL_BIZ", 10..=49 => "WHOLESALE", _ => "ENTERPRISE" })
        }
        "apply_tax" => {
            let amount = args.get(0).copied().unwrap_or(0);
            let rate = args.get(1).copied().unwrap_or(11);
            let tax = amount * rate / 100;
            format!("{{\"total\":{},\"tax\":{}}}", amount + tax, tax)
        }
        _ => format!("{{\"error\":\"unknown function: {}\"}}", func),
    };

    print!("{}", result);
}

fn extract_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find('"')?;
    Some(json[start..start + end].to_string())
}

fn extract_array_i32(json: &str, key: &str) -> Vec<i32> {
    let pattern = format!("\"{}\":[", key);
    let start = match json.find(&pattern) {
        Some(s) => s + pattern.len(),
        None => return vec![],
    };
    let end = match json[start..].find(']') {
        Some(e) => start + e,
        None => return vec![],
    };
    json[start..end]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect()
}
