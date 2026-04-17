// transform.wasm — Data transformation via WASI stdin/stdout
// Protocol: host sends JSON {"fn":"to_uppercase","data":"hello"} to stdin
//           WASM reads stdin, processes, writes result to stdout
// Build: rustc --target wasm32-wasip1 --edition 2021 transform.rs -o ../out/transform.wasm

fn main() {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).unwrap_or_default();

    let (func_name, data) = parse_envelope(&input);

    let output = match func_name.as_str() {
        "to_uppercase" => data.to_uppercase(),
        "reverse_bytes" => data.chars().rev().collect(),
        "count_vowels" => {
            let count = data.chars().filter(|c| "aeiouAEIOU".contains(*c)).count();
            format!("{}", count)
        }
        _ => format!("unknown function: {}", func_name),
    };

    print!("{}", output);
}

fn parse_envelope(input: &str) -> (String, String) {
    let func = extract_field(input, "fn").unwrap_or_default();
    let data = extract_field(input, "data").unwrap_or_default();
    (func, data)
}

fn extract_field(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
