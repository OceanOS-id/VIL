// 047 — Banking Transfer with Custom Error Stack (Hybrid: WASM Java for transfer, NativeCode for accounts)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/047-basic-custom-error-stack/vwfd/workflows", 8080)
        // Account listing — NativeCode (static data)
        .native("accounts_handler", |_| {
            Ok(json!({"accounts": [
                {"id": "ACC-1001", "name": "Alice", "balance_cents": 500000, "status": "active"},
                {"id": "ACC-1002", "name": "Bob", "balance_cents": 300000, "status": "active"},
                {"id": "ACC-1003", "name": "Charlie", "balance_cents": 100000, "status": "frozen"}
            ]}))
        })
        // Transfer processing — WASM Java (validation + domain errors, sandboxed)
        .wasm("transfer_handler", "examples/047-basic-custom-error-stack/vwfd/wasm/java/BankingTransfer.wasm")
        .run()
        .await;
}
