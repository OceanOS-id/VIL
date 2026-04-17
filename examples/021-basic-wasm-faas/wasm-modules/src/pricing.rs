// pricing.wasm — Business pricing rules (WASM sandboxed)
// Functions: calculate_price, apply_tax, bulk_discount
// Build: rustc --target wasm32-wasip1 -O pricing.rs -o ../out/pricing.wasm

#[no_mangle]
pub extern "C" fn calculate_price(base_cents: i32, quantity: i32) -> i32 {
    base_cents * quantity
}

#[no_mangle]
pub extern "C" fn apply_tax(amount_cents: i32, tax_rate_pct: i32) -> i32 {
    amount_cents + (amount_cents * tax_rate_pct / 100)
}

#[no_mangle]
pub extern "C" fn bulk_discount(total_cents: i32, quantity: i32) -> i32 {
    let discount_pct = match quantity {
        0..=4 => 0,
        5..=9 => 5,
        10..=49 => 10,
        _ => 20,
    };
    total_cents - (total_cents * discount_pct / 100)
}
