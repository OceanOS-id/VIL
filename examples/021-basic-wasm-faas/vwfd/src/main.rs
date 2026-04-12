// 021 — VWFD mode: WASM FaaS Business Rules
//
// Workflow: trigger → Function(wasm: pricing/calculate_price) → respond
//
// WASM module registered via .wasm() — sandboxed execution, separate memory.

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/021-basic-wasm-faas/vwfd/workflows", 3121)
        .wasm("pricing", "modules/pricing.wasm")
        .run()
        .await;
}
