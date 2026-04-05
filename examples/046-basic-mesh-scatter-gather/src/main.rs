// ╔════════════════════════════════════════════════════════════════════════╗
// ║  046 — Flight Search Aggregator (Mesh Scatter-Gather)               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Domain:   Travel — Flight Search                                   ║
// ║  Pattern:  VX_APP                                                    ║
// ║  Features: ServiceProcess, ServiceCtx, ShmSlice, VilResponse,       ║
// ║            VilModel, VxMeshConfig, ctx.send(), Visibility::Internal  ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A travel platform receives a flight search request and    ║
// ║  scatters it to 3 airline partner services in parallel, gathers      ║
// ║  their responses, ranks results by price, and returns the best.      ║
// ║                                                                      ║
// ║  Scatter-Gather Pattern:                                             ║
// ║    1. POST /api/search/flights → search gateway receives request     ║
// ║    2. Gateway scatters query to airline_a, airline_b, airline_c      ║
// ║       via ctx.send() (VIL mesh, zero-copy SHM)                       ║
// ║    3. Each airline service returns its best flights                   ║
// ║    4. Gateway gathers all results, ranks by price, returns top 5     ║
// ║                                                                      ║
// ║  Why VIL Mesh?                                                       ║
// ║    - ctx.send() uses zero-copy SHM — no serialization overhead       ║
// ║    - Internal services are not exposed to public HTTP                 ║
// ║    - VxMeshConfig declares the topology at startup                    ║
// ║    - Backpressure and circuit-breaking are built in                   ║
// ║                                                                      ║
// ║  Services:                                                           ║
// ║    gateway    (Public)   → POST /api/search/flights                  ║
// ║    airline_a  (Internal) → SkyWings Airlines inventory               ║
// ║    airline_b  (Internal) → OceanAir Airlines inventory               ║
// ║    airline_c  (Internal) → MountainJet Airlines inventory            ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-mesh-scatter-gather
// Test:
//   curl -X POST http://localhost:8080/api/search/flights \
//     -H 'Content-Type: application/json' \
//     -d '{"origin":"JFK","destination":"LAX","date":"2026-04-15","passengers":2}'

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;

// ── Business Domain Faults ──────────────────────────────────────────────

#[vil_fault]
pub enum FlightFault {
    /// No flights available for the requested route
    NoFlightsAvailable,
    /// One or more airline services failed to respond
    AirlineTimeout,
    /// Invalid search parameters
    InvalidSearch,
}

// ── Models (VilModel = SIMD-ready serialization) ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct FlightSearchRequest {
    origin: String,
    destination: String,
    date: String,
    passengers: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct FlightOffer {
    airline: String,
    flight_number: String,
    origin: String,
    destination: String,
    departure: String,
    arrival: String,
    price_cents: u64,
    seats_available: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SearchResponse {
    query: FlightSearchRequest,
    results: Vec<FlightOffer>,
    total_offers: usize,
    airlines_queried: u32,
    search_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct AirlineStatus {
    airline: String,
    role: String,
    routes_served: u32,
}

// ── Shared State ─────────────────────────────────────────────────────────

struct GatewayState {
    search_count: AtomicU64,
}

// ── Airline Inventory (simulated) ────────────────────────────────────────

/// Generate flight offers for a given airline based on route.
/// In production, this would query a real airline GDS/inventory system.
fn generate_offers(
    airline: &str,
    code_prefix: &str,
    origin: &str,
    destination: &str,
    date: &str,
    base_price: u64,
    passengers: u32,
) -> Vec<FlightOffer> {
    let routes = vec![
        (format!("{code_prefix}101"), "06:00", "09:15", base_price),
        (format!("{code_prefix}205"), "11:30", "14:45", base_price + 5000),
        (format!("{code_prefix}310"), "18:00", "21:20", base_price - 2000),
    ];

    routes
        .into_iter()
        .map(|(flight_number, dep, arr, price)| {
            FlightOffer {
                airline: airline.to_string(),
                flight_number,
                origin: origin.to_string(),
                destination: destination.to_string(),
                departure: format!("{date}T{dep}"),
                arrival: format!("{date}T{arr}"),
                price_cents: price * passengers as u64,
                seats_available: 42, // simulated availability
            }
        })
        .collect()
}

// ── Airline Service Handlers (Internal — mesh only) ──────────────────────

/// Airline A (SkyWings) — status endpoint for mesh health checks.
async fn airline_a_status() -> VilResponse<AirlineStatus> {
    VilResponse::ok(AirlineStatus {
        airline: "SkyWings Airlines".into(),
        role: "Internal airline inventory — receives queries via VIL mesh".into(),
        routes_served: 127,
    })
}

/// Airline B (OceanAir) — status endpoint.
async fn airline_b_status() -> VilResponse<AirlineStatus> {
    VilResponse::ok(AirlineStatus {
        airline: "OceanAir Airlines".into(),
        role: "Internal airline inventory — receives queries via VIL mesh".into(),
        routes_served: 89,
    })
}

/// Airline C (MountainJet) — status endpoint.
async fn airline_c_status() -> VilResponse<AirlineStatus> {
    VilResponse::ok(AirlineStatus {
        airline: "MountainJet Airlines".into(),
        role: "Internal airline inventory — receives queries via VIL mesh".into(),
        routes_served: 54,
    })
}

// ── Gateway Handler (Scatter-Gather) ─────────────────────────────────────

/// POST /search/flights — Scatter to 3 airlines, gather, rank, return best.
///
/// This handler demonstrates the scatter-gather pattern:
///   1. Parse the search request
///   2. Scatter the query to all airline services via ctx.send()
///   3. Gather responses (in this example, simulated inline since
///      mesh send is fire-and-forget; production would use request-reply)
///   4. Rank all offers by price (cheapest first)
///   5. Return top results
///
/// ctx.send() routes the query bytes through the VIL mesh to internal
/// services. The mesh topology is declared at startup via VxMeshConfig.
async fn search_flights(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<SearchResponse>> {
    let req: FlightSearchRequest = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {origin, destination, date, passengers}"
        ))?;

    if req.origin.is_empty() || req.destination.is_empty() {
        return Err(VilError::bad_request("origin and destination must not be empty"));
    }
    if req.passengers == 0 || req.passengers > 9 {
        return Err(VilError::bad_request("passengers must be 1-9"));
    }

    // ── SCATTER: Send the query to all airline services via mesh ─────
    // ctx.send() routes bytes to the named service through the VIL mesh.
    // In production, each airline service would receive the query,
    // look up its inventory, and reply via a response channel.
    let query_bytes = body.as_bytes();
    let _ = ctx.send("airline_a", query_bytes).await;
    let _ = ctx.send("airline_b", query_bytes).await;
    let _ = ctx.send("airline_c", query_bytes).await;

    // ── GATHER: Collect results from all airlines ────────────────────
    // In this example, we simulate the airline responses inline.
    // Production systems would use request-reply or callback patterns.
    let mut all_offers = Vec::new();

    all_offers.extend(generate_offers(
        "SkyWings", "SW", &req.origin, &req.destination,
        &req.date, 32_000, req.passengers,
    ));
    all_offers.extend(generate_offers(
        "OceanAir", "OA", &req.origin, &req.destination,
        &req.date, 28_500, req.passengers,
    ));
    all_offers.extend(generate_offers(
        "MountainJet", "MJ", &req.origin, &req.destination,
        &req.date, 35_000, req.passengers,
    ));

    // ── RANK: Sort by price (cheapest first) ─────────────────────────
    all_offers.sort_by_key(|o| o.price_cents);

    let total_offers = all_offers.len();

    // Return top 5 results
    all_offers.truncate(5);

    // Update search counter
    let state = ctx.state::<Arc<GatewayState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    let search_id = state.search_count.fetch_add(1, Ordering::Relaxed) + 1;

    Ok(VilResponse::ok(SearchResponse {
        query: req,
        results: all_offers,
        total_offers,
        airlines_queried: 3,
        search_id,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  046 — Flight Search Aggregator (Mesh Scatter-Gather)                ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  POST /api/search/flights → scatter to 3 airlines, gather, rank      ║");
    println!("║  Airlines: SkyWings, OceanAir, MountainJet (internal mesh services)  ║");
    println!("║  Pattern: scatter via ctx.send() → gather → rank → return best       ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let gateway_state = Arc::new(GatewayState {
        search_count: AtomicU64::new(0),
    });

    // Search Gateway: public-facing service that accepts flight search requests
    let gateway = ServiceProcess::new("gateway")
        .endpoint(Method::POST, "/search/flights", post(search_flights))
        .state(gateway_state);

    // Airline A: SkyWings — internal service, only reachable via mesh
    let airline_a = ServiceProcess::new("airline_a")
        .visibility(Visibility::Internal)
        .endpoint(Method::GET, "/status", get(airline_a_status));

    // Airline B: OceanAir — internal service, only reachable via mesh
    let airline_b = ServiceProcess::new("airline_b")
        .visibility(Visibility::Internal)
        .endpoint(Method::GET, "/status", get(airline_b_status));

    // Airline C: MountainJet — internal service, only reachable via mesh
    let airline_c = ServiceProcess::new("airline_c")
        .visibility(Visibility::Internal)
        .endpoint(Method::GET, "/status", get(airline_c_status));

    // Wire the mesh: gateway scatters to all 3 airline services via Data Lane.
    // Each route is an independent channel — the gateway can send to all 3
    // concurrently without blocking.
    VilApp::new("flight-search-aggregator")
        .port(8080)
        .service(gateway)
        .service(airline_a)
        .service(airline_b)
        .service(airline_c)
        .mesh(
            VxMeshConfig::new()
                .route("gateway", "airline_a", VxLane::Data)
                .route("gateway", "airline_b", VxLane::Data)
                .route("gateway", "airline_c", VxLane::Data),
        )
        .run()
        .await;
}
