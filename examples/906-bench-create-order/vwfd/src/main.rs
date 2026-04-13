// 906-bench-create-order — Benchmark workflow for head-to-head comparison
//
// Business logic: POST /api/orders → uuid_v4() + mean(prices) + sha256(email) → JSON
// All compute via built-in FaaS functions, zero custom handler code.
//
// Run:   cargo run --release
// Bench: hey -m POST -H 'Content-Type: application/json' \
//          -d '{"email":"alice@test.com","prices":[100,200,300]}' \
//          -c 10 -n 5000 http://localhost:8080/api/orders

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/906-bench-create-order/vwfd/workflows", 8080)
        .run()
        .await;
}
