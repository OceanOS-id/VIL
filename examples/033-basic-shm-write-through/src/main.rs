// ╔════════════════════════════════════════════════════════════════════════╗
// ║  033 — Real-time Analytics Dashboard (SHM Write-Through)            ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: VilResponse::with_shm(), ShmVilResponse,                  ║
// ║            ShmContext extractor, ExchangeHeap                        ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: An e-commerce API serves a product catalog. Each HTTP     ║
// ║  response is also written to shared memory (SHM) so that a          ║
// ║  co-located analytics service can read product access patterns       ║
// ║  without making a network round-trip.                                ║
// ║                                                                      ║
// ║  Why SHM write-through matters:                                      ║
// ║    - Analytics sidecar reads response data at memory speed (~50ns)   ║
// ║    - No extra HTTP call from analytics → catalog API                 ║
// ║    - No message queue needed for co-located services                 ║
// ║    - VilResponse::with_shm() does both HTTP + SHM in one call       ║
// ║                                                                      ║
// ║  Architecture:                                                       ║
// ║    [Customer] → HTTP → [Catalog API] → HTTP response to customer    ║
// ║                              ↓                                       ║
// ║                         SHM (ExchangeHeap)                           ║
// ║                              ↓                                       ║
// ║                    [Analytics Sidecar] reads at memory speed         ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-shm-write-through
// Test: curl -X POST http://localhost:8080/api/catalog/search \
//         -H 'Content-Type: application/json' \
//         -d '{"category":"electronics","max_price_cents":100000}'
//       curl http://localhost:8080/api/catalog/health

use vil_server::prelude::*;

// ── Business Domain Types ───────────────────────────────────────────────

/// A product in the e-commerce catalog.
#[derive(Clone, Serialize)]
struct Product {
    product_id: u64,
    name: String,
    category: String,
    price_cents: u64,
    stock_count: u32,
}

/// Search request from a customer browsing the catalog.
#[derive(Deserialize)]
struct CatalogSearchRequest {
    category: String,
    max_price_cents: u64,
}

/// Response sent to the customer AND written to SHM for analytics.
/// The analytics sidecar uses shm_available and products_returned
/// to track popular categories and price ranges.
#[derive(Serialize)]
struct CatalogResponse {
    products: Vec<Product>,
    products_returned: usize,
    category_searched: String,
    shm_available: bool,
    analytics_note: &'static str,
}

// ── Mock Product Database ───────────────────────────────────────────────
// In production, this would be a database query or cache lookup.
fn mock_catalog() -> Vec<Product> {
    vec![
        Product { product_id: 1001, name: "Wireless Mouse".into(), category: "electronics".into(), price_cents: 2999, stock_count: 150 },
        Product { product_id: 1002, name: "USB-C Hub".into(), category: "electronics".into(), price_cents: 4999, stock_count: 80 },
        Product { product_id: 1003, name: "4K Monitor".into(), category: "electronics".into(), price_cents: 49999, stock_count: 25 },
        Product { product_id: 2001, name: "Standing Desk".into(), category: "furniture".into(), price_cents: 89999, stock_count: 12 },
        Product { product_id: 2002, name: "Ergonomic Chair".into(), category: "furniture".into(), price_cents: 69999, stock_count: 30 },
    ]
}

// ── Catalog Search Handler (SHM Write-Through) ─────────────────────────

/// Search the product catalog by category and price.
///
/// KEY VIL FEATURE: VilResponse::with_shm()
/// This handler returns the response over HTTP to the customer AND writes
/// the serialized response to ExchangeHeap (shared memory). A co-located
/// analytics sidecar can then read this data at ~50ns latency instead of
/// making a separate network call.
///
/// ShmContext is automatically extracted from AppState's ExchangeHeap.
async fn catalog_search(
    State(state): State<AppState>,
    shm: ShmContext,
    body: ShmSlice,
) -> impl IntoResponse {
    let req: CatalogSearchRequest = body.json().expect("Invalid search JSON");

    // Filter products by category and price
    let products: Vec<Product> = mock_catalog()
        .into_iter()
        .filter(|p| p.category == req.category && p.price_cents <= req.max_price_cents)
        .collect();

    let products_returned = products.len();

    // Build the response — this goes to the customer via HTTP
    let response = CatalogResponse {
        products,
        products_returned,
        category_searched: req.category,
        shm_available: shm.available,
        analytics_note: "Response also written to SHM for co-located analytics sidecar",
    };

    // with_shm() does TWO things in one call:
    // 1. Serializes response as JSON → sends to customer via HTTP
    // 2. Writes the same bytes to ExchangeHeap → analytics sidecar reads at memory speed
    // Without VIL, you'd need: HTTP response + Kafka publish or Redis write = two hops.
    // With VIL: one call, zero network overhead for the analytics path.
    VilResponse::ok(response).with_shm(state.shm().clone())
}

/// Health check for the catalog service.
/// Demonstrates plain VilResponse (without SHM) for comparison.
async fn catalog_health(shm: ShmContext) -> VilResponse<CatalogResponse> {
    VilResponse::ok(CatalogResponse {
        products: vec![],
        products_returned: 0,
        category_searched: "n/a".into(),
        shm_available: shm.available,
        analytics_note: "Health check — plain VilResponse without SHM write-through",
    })
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  033 — Real-time Analytics Dashboard (SHM Write-Through)            ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  VilResponse::with_shm() → HTTP response + SHM write in one call    ║");
    println!("║  Analytics sidecar reads at memory speed (~50ns), no network hop     ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let catalog_svc = ServiceProcess::new("catalog")
        .endpoint(Method::POST, "/catalog/search", post(catalog_search))
        .endpoint(Method::GET, "/catalog/health", get(catalog_health));

    VilApp::new("realtime-analytics-dashboard")
        .port(8080)
        .service(catalog_svc)
        .run()
        .await;
}
