// validation.wasm — Order/payment validation rules (WASM sandboxed)
// Functions: validate_order, validate_age, validate_quantity
// Returns: 1 = valid, 0 = invalid
// Build: rustc --target wasm32-wasip1 -O validation.rs -o ../out/validation.wasm

#[no_mangle]
pub extern "C" fn validate_order(amount_cents: i32, max_allowed: i32) -> i32 {
    if amount_cents > 0 && amount_cents <= max_allowed { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn validate_age(age: i32, min_age: i32) -> i32 {
    if age >= min_age { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn validate_quantity(quantity: i32, max_qty: i32) -> i32 {
    if quantity > 0 && quantity <= max_qty { 1 } else { 0 }
}
