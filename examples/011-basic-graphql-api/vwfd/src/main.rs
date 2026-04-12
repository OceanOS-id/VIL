// 011 — GraphQL API (Hybrid: WASM AssemblyScript for query, NativeCode for schema metadata)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/011-basic-graphql-api/vwfd/workflows", 8080)
        // Static schema metadata — NativeCode (trivial, no computation)
        .native("gql_schema", |_| {
            Ok(json!({
                "types": ["Order", "Customer", "Product"],
                "queries": ["orders", "customers", "products"],
                "mutations": ["createOrder", "updateOrder"],
            }))
        })
        .native("gql_entities", |_| {
            Ok(json!([
                {"name": "Order", "fields": ["id", "customer_id", "total", "status"]},
                {"name": "Customer", "fields": ["id", "name", "email"]},
                {"name": "Product", "fields": ["id", "name", "price"]},
            ]))
        })
        // Product query — WASM AssemblyScript (sandboxed, language-diverse)
        .wasm("gql_query", "examples/011-basic-graphql-api/vwfd/wasm/assemblyscript/products.wasm")
        .run()
        .await;
}
