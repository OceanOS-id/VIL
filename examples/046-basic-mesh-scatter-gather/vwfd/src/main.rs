// 046 — Flight Search Scatter-Gather (NativeCode — mesh routing simulated)
// Business logic matches standard src/main.rs:
//   - 3 airlines: SkyWings (SW, base=32000), OceanAir (OA, base=28500), MountainJet (MJ, base=35000)
//   - 3 flights per airline: routes 101/205/310 with dep times 06:00/11:30/18:00
//   - Price multiplied by passengers, sorted cheapest first, top 5
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static SEARCH_COUNT: AtomicU64 = AtomicU64::new(0);

struct Airline {
    code: &'static str,
    name: &'static str,
    base_price: i64,
}

const AIRLINES: &[Airline] = &[
    Airline { code: "SW", name: "SkyWings", base_price: 32_000 },
    Airline { code: "OA", name: "OceanAir", base_price: 28_500 },
    Airline { code: "MJ", name: "MountainJet", base_price: 35_000 },
];

const ROUTES: &[(u16, &str, &str, &str, &str)] = &[
    (101, "06:00", "09:15", "morning", "0"),
    (205, "11:30", "14:45", "midday", "5000"),
    (310, "18:00", "21:20", "evening", "-2000"),
];

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/046-basic-mesh-scatter-gather/vwfd/workflows", 8080)
        .native("mesh_flight_scatter", |input| {
            let body = &input["body"];
            let origin = body["origin"].as_str().unwrap_or("");
            let destination = body["destination"].as_str().unwrap_or("");
            let date = body["date"].as_str().unwrap_or("2026-04-15");
            let passengers = body["passengers"].as_i64().unwrap_or(1);

            // Validation
            if origin.is_empty() || destination.is_empty() {
                return Ok(json!({"error": "origin and destination are required"}));
            }
            if passengers < 1 || passengers > 9 {
                return Ok(json!({"error": "passengers must be 1-9"}));
            }

            // Generate offers from all airlines
            let mut offers = Vec::new();
            for airline in AIRLINES {
                for &(route, dep, arr, _slot, price_adj) in ROUTES {
                    let adj: i64 = price_adj.parse().unwrap_or(0);
                    let price_cents = (airline.base_price + adj) * passengers;
                    offers.push(json!({
                        "airline": airline.name,
                        "flight_number": format!("{}{}", airline.code, route),
                        "origin": origin,
                        "destination": destination,
                        "departure": format!("{}T{}", date, dep),
                        "arrival": format!("{}T{}", date, arr),
                        "price_cents": price_cents,
                        "seats_available": 42
                    }));
                }
            }

            // Sort by price (cheapest first)
            offers.sort_by_key(|o| o["price_cents"].as_i64().unwrap_or(0));
            // Top 5
            offers.truncate(5);

            let search_id = SEARCH_COUNT.fetch_add(1, Ordering::Relaxed);

            Ok(json!({
                "query": { "origin": origin, "destination": destination, "date": date, "passengers": passengers },
                "results": offers,
                "total_offers": offers.len(),
                "airlines_queried": 3,
                "search_id": search_id
            }))
        })
        .run().await;
}
