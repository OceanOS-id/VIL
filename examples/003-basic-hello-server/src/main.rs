// ╔════════════════════════════════════════════════════════════╗
// ║  003 — Currency Exchange Service                          ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Finance — Foreign Exchange / Remittance         ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: ServiceProcess, ServiceCtx, ShmSlice,          ║
// ║            VilResponse, VilModel, .observer(true)          ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Currency conversion API for money changers.     ║
// ║  10 currency pairs (IDR base), buy/sell spread, atomic     ║
// ║  conversion counter for volume tracking.                    ║
// ║                                                             ║
// ║  Demonstrates what makes VIL different from plain Axum:    ║
// ║    1. ServiceProcess (not Router)                           ║
// ║    2. ShmSlice zero-copy body (not Json<T>)                 ║
// ║    3. ServiceCtx for state (not Extension<T>)               ║
// ║    4. VilResponse<T> typed envelope (not Json<Value>)       ║
// ║    5. VilModel derive (SIMD-ready serialization)            ║
// ║    6. .observer(true) embedded dashboard                    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-hello-server
// Test:
//   curl http://localhost:8080/api/fx/rates
//   curl -X POST http://localhost:8080/api/fx/convert \
//     -H 'Content-Type: application/json' \
//     -d '{"from":"USD","to":"IDR","amount":100.0}'
//   curl http://localhost:8080/api/fx/stats

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;

// ── Exchange Rates (IDR base) ────────────────────────────────────────────
// Mid-market rates. Buy = mid * (1 - spread/2), Sell = mid * (1 + spread/2).

struct CurrencyRate {
    code: &'static str,
    name: &'static str,
    mid_rate: f64,     // 1 unit = X IDR
    spread_pct: f64,   // buy/sell spread percentage
}

const RATES: &[CurrencyRate] = &[
    CurrencyRate { code: "USD", name: "US Dollar", mid_rate: 15_850.0, spread_pct: 1.0 },
    CurrencyRate { code: "EUR", name: "Euro", mid_rate: 17_200.0, spread_pct: 1.2 },
    CurrencyRate { code: "SGD", name: "Singapore Dollar", mid_rate: 11_800.0, spread_pct: 1.5 },
    CurrencyRate { code: "MYR", name: "Malaysian Ringgit", mid_rate: 3_560.0, spread_pct: 2.0 },
    CurrencyRate { code: "JPY", name: "Japanese Yen", mid_rate: 105.0, spread_pct: 1.8 },
    CurrencyRate { code: "AUD", name: "Australian Dollar", mid_rate: 10_300.0, spread_pct: 1.5 },
    CurrencyRate { code: "GBP", name: "British Pound", mid_rate: 20_100.0, spread_pct: 1.0 },
    CurrencyRate { code: "CNY", name: "Chinese Yuan", mid_rate: 2_180.0, spread_pct: 2.5 },
    CurrencyRate { code: "THB", name: "Thai Baht", mid_rate: 460.0, spread_pct: 2.0 },
    CurrencyRate { code: "SAR", name: "Saudi Riyal", mid_rate: 4_225.0, spread_pct: 1.5 },
];

fn find_rate(code: &str) -> Option<&'static CurrencyRate> {
    RATES.iter().find(|r| r.code.eq_ignore_ascii_case(code))
}

// ── Models (VilModel = SIMD-ready serialization) ─────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RateInfo {
    code: String,
    name: String,
    buy_rate: f64,
    sell_rate: f64,
    mid_rate: f64,
    spread_pct: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RatesResponse {
    base: String,
    rates: Vec<RateInfo>,
    updated_at: u64,
}

#[derive(Debug, Deserialize)]
struct ConvertRequest {
    from: String,
    to: String,
    amount: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ConvertResponse {
    from: String,
    to: String,
    amount: f64,
    rate_applied: f64,
    converted_amount: f64,
    spread_pct: f64,
    conversion_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct StatsResponse {
    total_conversions: u64,
    total_volume_idr: u64,
    uptime_secs: u64,
}

// ── Shared State (via ServiceCtx, not Extension<T>) ──────────────────────

struct FxState {
    conversion_count: AtomicU64,
    volume_idr: AtomicU64,
    started_at: std::time::Instant,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// GET /rates — All exchange rates with buy/sell spread.
async fn rates() -> VilResponse<RatesResponse> {
    let rates: Vec<RateInfo> = RATES.iter().map(|r| {
        let half_spread = r.spread_pct / 200.0;
        RateInfo {
            code: r.code.into(),
            name: r.name.into(),
            buy_rate: r.mid_rate * (1.0 - half_spread),
            sell_rate: r.mid_rate * (1.0 + half_spread),
            mid_rate: r.mid_rate,
            spread_pct: r.spread_pct,
        }
    }).collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    VilResponse::ok(RatesResponse { base: "IDR".into(), rates, updated_at: now })
}

/// POST /convert — Convert between currencies with buy/sell spread.
///
/// ShmSlice: zero-copy body from ExchangeHeap (not Json<T>).
/// ServiceCtx: state access via ctx.state (not Extension<T>).
async fn convert(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ConvertResponse>> {
    let req: ConvertRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {from, to, amount}"))?;

    if req.amount <= 0.0 {
        return Err(VilError::bad_request("amount must be positive"));
    }

    // Convert: from → IDR → to
    let (idr_amount, from_spread) = if req.from.eq_ignore_ascii_case("IDR") {
        (req.amount, 0.0)
    } else {
        let rate = find_rate(&req.from)
            .ok_or_else(|| VilError::not_found(format!("currency {} not supported", req.from)))?;
        let buy_rate = rate.mid_rate * (1.0 - rate.spread_pct / 200.0);
        (req.amount * buy_rate, rate.spread_pct)
    };

    let (converted, to_spread) = if req.to.eq_ignore_ascii_case("IDR") {
        (idr_amount, 0.0)
    } else {
        let rate = find_rate(&req.to)
            .ok_or_else(|| VilError::not_found(format!("currency {} not supported", req.to)))?;
        let sell_rate = rate.mid_rate * (1.0 + rate.spread_pct / 200.0);
        (idr_amount / sell_rate, rate.spread_pct)
    };

    let rate_applied = if req.amount > 0.0 { converted / req.amount } else { 0.0 };

    // ServiceCtx state (not Extension<T>)
    let state = ctx.state::<Arc<FxState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    let conversion_id = state.conversion_count.fetch_add(1, Ordering::Relaxed) + 1;
    state.volume_idr.fetch_add(idr_amount as u64, Ordering::Relaxed);

    Ok(VilResponse::ok(ConvertResponse {
        from: req.from.to_uppercase(),
        to: req.to.to_uppercase(),
        amount: req.amount,
        rate_applied,
        converted_amount: (converted * 100.0).round() / 100.0,
        spread_pct: from_spread.max(to_spread),
        conversion_id,
    }))
}

/// GET /stats — Conversion volume statistics.
async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<StatsResponse>> {
    let state = ctx.state::<Arc<FxState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    Ok(VilResponse::ok(StatsResponse {
        total_conversions: state.conversion_count.load(Ordering::Relaxed),
        total_volume_idr: state.volume_idr.load(Ordering::Relaxed),
        uptime_secs: state.started_at.elapsed().as_secs(),
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let fx_state = Arc::new(FxState {
        conversion_count: AtomicU64::new(0),
        volume_idr: AtomicU64::new(0),
        started_at: std::time::Instant::now(),
    });

    let fx = ServiceProcess::new("fx")
        .endpoint(Method::GET, "/rates", get(rates))
        .endpoint(Method::POST, "/convert", post(convert))
        .endpoint(Method::GET, "/stats", get(stats))
        .state(fx_state);

    VilApp::new("currency-exchange")
        .port(8080)
        .observer(true)
        .service(fx)
        .run()
        .await;
}
