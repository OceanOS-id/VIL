// ╔════════════════════════════════════════════════════════════════════════╗
// ║  038 — Restaurant Order System (vil_app! Declarative Macro)         ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: vil_app! macro — generates main() + VilApp + routing      ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A restaurant management system with four endpoints:       ║
// ║    - GET  /menu           → browse the menu (appetizers, mains, etc.)║
// ║    - POST /order          → place a new food order                   ║
// ║    - GET  /order/:id      → check order status by ID                 ║
// ║    - GET  /kitchen/status → current kitchen load and wait times      ║
// ║                                                                      ║
// ║  Why vil_app! matters:                                               ║
// ║    - Replaces manual VilApp::new() + ServiceProcess + #[tokio::main] ║
// ║    - Reads like a config file — even non-Rust developers can         ║
// ║      understand the endpoint layout at a glance                      ║
// ║    - Reduces boilerplate from ~15 lines to ~5 lines for app wiring  ║
// ║    - The macro generates identical code to manual setup — no         ║
// ║      performance difference, just ergonomic improvement              ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-vil-app-dsl
// Test: curl http://localhost:8080/menu
//       curl -X POST http://localhost:8080/order \
//         -H 'Content-Type: application/json' \
//         -d '{"table_number":7,"items":["margherita_pizza","caesar_salad","tiramisu"]}'
//       curl http://localhost:8080/order/7042
//       curl http://localhost:8080/kitchen/status

use vil_server::prelude::*;

// ── Menu Domain ─────────────────────────────────────────────────────────

/// A menu item with name, price, and category.
#[derive(Serialize)]
struct MenuItem {
    name: &'static str,
    price_cents: u64,
    category: &'static str,
}

/// Full menu response with all available items.
#[derive(Serialize)]
struct MenuResponse {
    restaurant: &'static str,
    items: Vec<MenuItem>,
    specials_today: &'static str,
}

// ── Order Domain ────────────────────────────────────────────────────────

/// Order placed by a customer at a table.
#[derive(Deserialize)]
struct PlaceOrderRequest {
    table_number: u32,
    items: Vec<String>,
}

/// Confirmed order with estimated preparation time.
#[derive(Serialize)]
struct OrderConfirmation {
    order_id: u64,
    table_number: u32,
    items_ordered: Vec<String>,
    total_cents: u64,
    estimated_minutes: u32,
    status: &'static str,
}

/// Order status lookup result.
#[derive(Serialize)]
struct OrderStatus {
    order_id: String,
    status: &'static str,
    progress_percent: u8,
    estimated_remaining_minutes: u32,
}

// ── Kitchen Domain ──────────────────────────────────────────────────────

/// Current kitchen workload and capacity.
#[derive(Serialize)]
struct KitchenStatus {
    orders_in_queue: u32,
    orders_cooking: u32,
    chefs_on_duty: u32,
    avg_wait_minutes: u32,
    kitchen_load_percent: u8,
}

// ── Handler Implementations ─────────────────────────────────────────────

/// Browse the restaurant menu.
/// Returns all available items grouped by category.
async fn menu() -> VilResponse<MenuResponse> {
    VilResponse::ok(MenuResponse {
        restaurant: "Trattoria VIL — Italian Kitchen",
        items: vec![
            MenuItem {
                name: "Bruschetta",
                price_cents: 899,
                category: "appetizer",
            },
            MenuItem {
                name: "Caesar Salad",
                price_cents: 1299,
                category: "appetizer",
            },
            MenuItem {
                name: "Margherita Pizza",
                price_cents: 1599,
                category: "main",
            },
            MenuItem {
                name: "Spaghetti Carbonara",
                price_cents: 1899,
                category: "main",
            },
            MenuItem {
                name: "Grilled Salmon",
                price_cents: 2499,
                category: "main",
            },
            MenuItem {
                name: "Tiramisu",
                price_cents: 999,
                category: "dessert",
            },
            MenuItem {
                name: "Panna Cotta",
                price_cents: 899,
                category: "dessert",
            },
        ],
        specials_today: "Chef's special: Truffle Risotto ($22.99)",
    })
}

/// Place a new food order.
/// Calculates total based on ordered items and estimates prep time.
async fn place_order(body: ShmSlice) -> Result<VilResponse<OrderConfirmation>, VilError> {
    let req: PlaceOrderRequest = body
        .json()
        .map_err(|_| VilError::bad_request("Invalid order — need table_number and items array"))?;

    if req.items.is_empty() {
        return Err(VilError::bad_request(
            "Order must contain at least one item",
        ));
    }

    // Calculate total (simplified — in production: lookup menu prices)
    let price_per_item = 1500u64; // average $15.00
    let total_cents = req.items.len() as u64 * price_per_item;

    // Estimate prep time: 10 min base + 3 min per additional item
    let estimated_minutes = 10 + (req.items.len().saturating_sub(1) as u32) * 3;

    // Generate order ID from table number
    let order_id = req.table_number as u64 * 1000 + 42;

    Ok(VilResponse::ok(OrderConfirmation {
        order_id,
        table_number: req.table_number,
        items_ordered: req.items,
        total_cents,
        estimated_minutes,
        status: "received — sent to kitchen",
    }))
}

/// Check order status by ID.
/// In production: look up from database or kitchen display system.
async fn order_status(Path(id): Path<String>) -> Result<VilResponse<OrderStatus>, VilError> {
    let order_id = id;
    Ok(VilResponse::ok(OrderStatus {
        order_id,
        status: "cooking — your pizza is in the oven",
        progress_percent: 65,
        estimated_remaining_minutes: 8,
    }))
}

/// Current kitchen status — shows workload and wait times.
/// Restaurant managers use this to decide if they need more staff.
async fn kitchen_status() -> VilResponse<KitchenStatus> {
    // Demo values — in production: query kitchen display system or order queue
    VilResponse::ok(KitchenStatus {
        orders_in_queue: 5,     // demo value
        orders_cooking: 3,      // demo value
        chefs_on_duty: 2,       // demo value
        avg_wait_minutes: 18,   // demo value
        kitchen_load_percent: 75, // demo value
    })
}

// ── vil_app! DSL ────────────────────────────────────────────────────────
// The entire app is declared in 7 lines. vil_app! generates:
//   - #[tokio::main] async fn main()
//   - ServiceProcess::new(...) with all endpoints
//   - VilApp::new(...).port(...).service(...).run().await
//
// Compare this to manual setup (~15+ lines of boilerplate).
// The macro produces identical runtime code — zero overhead.
vil_app! {
    name: "restaurant-order-system",
    port: 8080,
    endpoints: {
        GET  "/menu"           => menu,
        POST "/order"          => place_order,
        GET  "/order/:id"      => order_status,
        GET  "/kitchen/status" => kitchen_status,
    }
}
