// ╔════════════════════════════════════════════════════════════════════════╗
// ║  030 — E-Commerce Order Pipeline (Tri-Lane Messaging)               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: ctx.send(), ctx.trigger(), ctx.control()                  ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: Customer places an order through the API gateway.         ║
// ║  The gateway orchestrates fulfillment using VIL's Tri-Lane           ║
// ║  messaging — three independent channels that never block each other. ║
// ║                                                                      ║
// ║  Business Flow:                                                      ║
// ║    1. POST /api/orders/place → OrderGateway receives order           ║
// ║    2. ctx.trigger("fulfillment", ...) → Signal to start processing   ║
// ║    3. ctx.send("fulfillment", order_bytes) → Send order data         ║
// ║       (Data Lane, zero-copy via SHM)                                 ║
// ║    4. ctx.control("fulfillment", ...) → Request inventory            ║
// ║       reservation (Control Lane — never blocked by Data congestion)  ║
// ║                                                                      ║
// ║  Why Tri-Lane matters in e-commerce:                                 ║
// ║    - Trigger Lane: "Wake up, new order incoming" (lightweight)       ║
// ║    - Data Lane: Full order payload (could be large, zero-copy)       ║
// ║    - Control Lane: Inventory reservation command (must not be         ║
// ║      blocked even if data channel is congested during flash sales)   ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-trilane-messaging
// Test: curl -X POST http://localhost:8080/api/orders/place \
//         -H 'Content-Type: application/json' \
//         -d '{"customer_id":1001,"product_id":42,"quantity":2,"amount_cents":9999}'

use vil_server::prelude::*;

// ── Business Domain Faults ──────────────────────────────────────────────
// #[vil_fault] generates VIL-compatible error types with fault codes.
// Each variant represents a real failure mode in order processing.
#[vil_fault]
pub enum OrderFault {
    /// Payment gateway declined the charge (e.g., insufficient funds)
    PaymentDeclined,
    /// Warehouse has zero stock for the requested product
    OutOfStock,
    /// Fulfillment service did not respond within SLA (e.g., 5 seconds)
    FulfillmentTimeout,
}

// ── Business Domain Request ─────────────────────────────────────────────
// The JSON body a customer sends when placing an order.
// In production, this would include shipping address, payment token, etc.
#[derive(Deserialize)]
struct PlaceOrderRequest {
    customer_id: u64,
    product_id: u64,
    quantity: u32,
    amount_cents: u64,
}

// ── Business Domain Response ────────────────────────────────────────────
// Confirmation returned to the customer after the order is accepted.
// The order_id is generated server-side; fulfillment_notified confirms
// that the warehouse was successfully signaled via Tri-Lane messaging.
#[derive(Serialize)]
struct OrderConfirmation {
    order_id: u64,
    customer_id: u64,
    product_id: u64,
    quantity: u32,
    amount_cents: u64,
    bytes_sent_to_fulfillment: usize,
    fulfillment_notified: bool,
    inventory_reserved: bool,
}

// ── Fulfillment Service Status ──────────────────────────────────────────
// Internal status endpoint for the fulfillment service.
// Only accessible via mesh (not exposed to public HTTP).
#[derive(Serialize)]
struct FulfillmentStatus {
    service: &'static str,
    role: &'static str,
    warehouse_region: &'static str,
    capacity_percent: u8,
}

/// Order Gateway Handler — uses all three Tri-Lane methods.
///
/// This is the core of the example: a single handler that demonstrates
/// how VIL's three messaging lanes work together in a real business flow.
///
/// - Trigger Lane: lightweight "wake up" signal (no payload needed)
/// - Data Lane: sends the full order payload via zero-copy SHM
/// - Control Lane: sends inventory reservation command (priority channel)
async fn place_order(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<OrderConfirmation>, VilError> {
    // Parse the incoming order from the HTTP request body.
    // ShmSlice provides zero-copy access to the request bytes.
    let order: PlaceOrderRequest = body.json()
        .map_err(|_| VilError::bad_request("Invalid order JSON — expected customer_id, product_id, quantity, amount_cents"))?;

    // Generate a simple order ID (in production: use a distributed ID generator)
    let order_id = order.customer_id * 10000 + order.product_id;

    // ── STEP 1: Trigger Lane ────────────────────────────────────────
    // Signal the fulfillment service to prepare for an incoming order.
    // Trigger is lightweight — it carries a small marker, not the full payload.
    // Think of it as ringing the warehouse doorbell before the delivery truck arrives.
    ctx.trigger("fulfillment", b"new_order").await?;

    // ── STEP 2: Data Lane ───────────────────────────────────────────
    // Send the full order payload to fulfillment via the Data Lane.
    // This uses zero-copy SHM: the fulfillment service reads the same
    // memory region without copying bytes over a network socket.
    // During flash sales, this lane might be congested — but that's OK
    // because the Control Lane (below) is completely independent.
    let order_bytes = body.as_bytes();
    let bytes_sent = ctx.send("fulfillment", order_bytes).await?;

    // ── STEP 3: Control Lane ────────────────────────────────────────
    // Request inventory reservation via the Control Lane.
    // Control Lane is NEVER blocked by Data Lane congestion.
    // This is critical: even if 10,000 order payloads are queued on Data,
    // the "reserve inventory" command gets through immediately.
    // In banking, this pattern prevents control signals (like "halt trading")
    // from being stuck behind bulk data transfers.
    let reserve_cmd = format!("reserve:{}:{}", order.product_id, order.quantity);
    ctx.control("fulfillment", reserve_cmd.as_bytes()).await?;

    // Return confirmation to the customer
    Ok(VilResponse::ok(OrderConfirmation {
        order_id,
        customer_id: order.customer_id,
        product_id: order.product_id,
        quantity: order.quantity,
        amount_cents: order.amount_cents,
        bytes_sent_to_fulfillment: bytes_sent,
        fulfillment_notified: true,
        inventory_reserved: true,
    }))
}

/// Fulfillment service status endpoint (internal mesh-only service).
/// Not exposed to public HTTP — only reachable via VIL mesh routing.
/// In production, this would report warehouse capacity, pending orders, etc.
async fn fulfillment_status() -> VilResponse<FulfillmentStatus> {
    VilResponse::ok(FulfillmentStatus {
        service: "fulfillment",
        role: "Internal warehouse service — receives orders via Tri-Lane mesh",
        warehouse_region: "us-west-2",
        capacity_percent: 73,
    })
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  030 — E-Commerce Order Pipeline (Tri-Lane Messaging)               ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Trigger Lane → \"wake up, new order\"                                ║");
    println!("║  Data Lane    → full order payload (zero-copy SHM)                   ║");
    println!("║  Control Lane → inventory reservation (never blocked by Data)        ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // OrderGateway: public-facing service that accepts customer orders
    let order_gateway = ServiceProcess::new("gateway")
        .endpoint(Method::POST, "/orders/place", post(place_order));

    // Fulfillment: internal service that processes orders and manages inventory
    // Visibility::Internal means it is only reachable via mesh, not public HTTP
    let fulfillment = ServiceProcess::new("fulfillment")
        .visibility(Visibility::Internal)
        .endpoint(Method::GET, "/status", get(fulfillment_status));

    // Wire the mesh: gateway sends to fulfillment via all three lanes.
    // Each lane is an independent channel — congestion on one never blocks another.
    VilApp::new("ecommerce-order-pipeline")
        .port(8080)
        .service(order_gateway)
        .service(fulfillment)
        .mesh(VxMeshConfig::new()
            .route("gateway", "fulfillment", VxLane::Trigger)   // "wake up" signal
            .route("gateway", "fulfillment", VxLane::Data)      // order payload
            .route("gateway", "fulfillment", VxLane::Control))  // inventory reservation
        .run()
        .await;
}
