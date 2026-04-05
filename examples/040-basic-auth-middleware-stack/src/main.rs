// ╔════════════════════════════════════════════════════════════╗
// ║  040 — Multi-Tenant API Auth Middleware Stack             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   SaaS Platform — Multi-tenant API               ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: JwtAuth (manual Bearer check), RateLimit,      ║
// ║            ServiceCtx, ShmSlice, VilResponse, VilModel    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: API with JWT auth, per-tenant rate limiting,   ║
// ║  and CSRF protection pattern. Demonstrates VIL auth       ║
// ║  middleware stack without external dependencies.           ║
// ║                                                           ║
// ║  Endpoints:                                               ║
// ║    POST /api/admin/login     → JWT token (demo creds)     ║
// ║    GET  /api/protected/data  → requires Bearer, rated     ║
// ║    GET  /api/public/info     → no auth required           ║
// ║    GET  /api/protected/tenant→ tenant_id from JWT claims  ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-auth-middleware-stack
// Test:
//   curl http://localhost:8080/api/public/info
//   curl -X POST http://localhost:8080/api/admin/login \
//     -H 'Content-Type: application/json' \
//     -d '{"username":"admin","password":"vil-demo"}'
//   curl http://localhost:8080/api/protected/data \
//     -H 'Authorization: Bearer <token-from-login>'
//   curl http://localhost:8080/api/protected/tenant \
//     -H 'Authorization: Bearer <token-from-login>'

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct LoginResponse {
    token: String,
    token_type: String,
    expires_in_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ProtectedData {
    message: String,
    request_count: u64,
    rate_limit_remaining: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PublicInfo {
    service: String,
    version: String,
    status: String,
    auth_required_endpoints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TenantInfo {
    tenant_id: String,
    username: String,
    roles: Vec<String>,
    issued_at: u64,
}

// ── JWT helpers (lightweight, no external crate) ─────────────────────────
// For demo purposes we use a simple base64-encoded JSON token.
// Production would use vil_server_auth::JwtAuth with jsonwebtoken crate.

const JWT_SECRET: &str = "vil-demo-secret-2026";
const DEMO_USERNAME: &str = "admin";
const DEMO_PASSWORD: &str = "vil-demo";
const TOKEN_EXPIRY_SECS: u64 = 3600;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    tenant_id: String,
    roles: Vec<String>,
    iat: u64,
    exp: u64,
}

/// Encode claims as a simple base64 JSON token (demo only).
fn encode_token(claims: &JwtClaims) -> String {
    let header = base64_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let payload_bytes = serde_json::to_vec(claims).unwrap_or_default();
    let payload = base64_encode(&payload_bytes);
    // For demo: signature = base64 of header.payload.secret
    let signature = base64_encode(
        format!("{}.{}.{}", header, payload, JWT_SECRET).as_bytes(),
    );
    format!("{}.{}.{}", header, payload, signature)
}

/// Decode and validate a demo JWT token. Returns claims on success.
fn decode_token(token: &str) -> Result<JwtClaims, VilError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(VilError::unauthorized("malformed token"));
    }

    let payload_bytes = base64_decode(parts[1])
        .map_err(|_| VilError::unauthorized("invalid token encoding"))?;

    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|_| VilError::unauthorized("invalid token payload"))?;

    // Verify signature
    let expected_sig = base64_encode(
        format!("{}.{}.{}", parts[0], parts[1], JWT_SECRET).as_bytes(),
    );
    if parts[2] != expected_sig {
        return Err(VilError::unauthorized("invalid token signature"));
    }

    // Check expiry
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if claims.exp < now {
        return Err(VilError::unauthorized("token expired"));
    }

    Ok(claims)
}

/// Extract Bearer token from Authorization header.
fn extract_bearer(headers: &vil_server::axum::http::HeaderMap) -> Result<String, VilError> {
    let auth_value = headers
        .get("authorization")
        .ok_or_else(|| VilError::unauthorized("missing Authorization header"))?
        .to_str()
        .map_err(|_| VilError::unauthorized("invalid Authorization header encoding"))?;

    let token = auth_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| VilError::unauthorized("expected Bearer token format"))?;

    Ok(token.to_string())
}

// Simple base64 helpers (no external crate needed for demo)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const DECODE: [u8; 128] = {
        let mut table = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let bytes: Vec<u8> = input.bytes().collect();

    for chunk in bytes.chunks(4) {
        let mut acc: u32 = 0;
        let len = chunk.len();
        for (i, &b) in chunk.iter().enumerate() {
            if b >= 128 || DECODE[b as usize] == 255 {
                return Err("invalid base64");
            }
            acc |= (DECODE[b as usize] as u32) << (6 * (3 - i));
        }
        result.push((acc >> 16) as u8);
        if len > 2 {
            result.push((acc >> 8) as u8);
        }
        if len > 3 {
            result.push(acc as u8);
        }
    }
    Ok(result)
}

// ── Rate Limiter State ──────────────────────────────────────────────────

struct RateBucket {
    count: AtomicU64,
    window_start: RwLock<Instant>,
}

struct AuthState {
    /// Per-tenant rate limiting: tenant_id -> bucket
    rate_buckets: RwLock<HashMap<String, Arc<RateBucket>>>,
    /// Global request counter
    total_requests: AtomicU64,
    /// Max requests per minute per tenant
    max_requests_per_min: u64,
}

impl AuthState {
    fn new(max_requests_per_min: u64) -> Self {
        Self {
            rate_buckets: RwLock::new(HashMap::new()),
            total_requests: AtomicU64::new(0),
            max_requests_per_min,
        }
    }

    /// Check rate limit for a given key. Returns remaining count or error.
    fn check_rate_limit(&self, key: &str) -> Result<u64, VilError> {
        let now = Instant::now();

        // Get or create bucket
        let bucket = {
            let buckets = self.rate_buckets.read().unwrap();
            buckets.get(key).cloned()
        };

        let bucket = match bucket {
            Some(b) => b,
            None => {
                let b = Arc::new(RateBucket {
                    count: AtomicU64::new(0),
                    window_start: RwLock::new(now),
                });
                self.rate_buckets.write().unwrap().insert(key.to_string(), b.clone());
                b
            }
        };

        // Check if window has expired (60 seconds)
        {
            let window_start = *bucket.window_start.read().unwrap();
            if now.duration_since(window_start).as_secs() >= 60 {
                bucket.count.store(0, Ordering::Relaxed);
                *bucket.window_start.write().unwrap() = now;
            }
        }

        let current = bucket.count.fetch_add(1, Ordering::Relaxed);
        if current >= self.max_requests_per_min {
            return Err(VilError::rate_limited());
        }

        Ok(self.max_requests_per_min - current - 1)
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /login — authenticate with demo credentials, return JWT token.
async fn login_handler(body: ShmSlice) -> HandlerResult<VilResponse<LoginResponse>> {
    let req: LoginRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected username and password"))?;

    if req.username != DEMO_USERNAME || req.password != DEMO_PASSWORD {
        return Err(VilError::unauthorized("invalid credentials"));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let claims = JwtClaims {
        sub: req.username.clone(),
        tenant_id: format!("tenant-{}", req.username),
        roles: vec!["admin".into(), "read".into(), "write".into()],
        iat: now,
        exp: now + TOKEN_EXPIRY_SECS,
    };

    let token = encode_token(&claims);

    Ok(VilResponse::ok(LoginResponse {
        token,
        token_type: "Bearer".into(),
        expires_in_secs: TOKEN_EXPIRY_SECS,
    }))
}

/// GET /data — protected endpoint, requires valid Bearer token, rate limited (10 req/min).
async fn protected_data(
    ctx: ServiceCtx,
    headers: vil_server::axum::http::HeaderMap,
) -> HandlerResult<VilResponse<ProtectedData>> {
    // Step 1: Extract and validate JWT
    let token = extract_bearer(&headers)?;
    let claims = decode_token(&token)?;

    // Step 2: Check rate limit per tenant (10 req/min)
    let state = ctx.state::<Arc<AuthState>>()
        .map_err(|_| VilError::internal("auth state not found"))?;

    let remaining = state.check_rate_limit(&claims.tenant_id)?;
    let total = state.total_requests.fetch_add(1, Ordering::Relaxed) + 1;

    Ok(VilResponse::ok(ProtectedData {
        message: format!("Hello, {}! You have access to protected data.", claims.sub),
        request_count: total,
        rate_limit_remaining: remaining,
    }))
}

/// GET /info — public endpoint, no auth required.
async fn public_info() -> VilResponse<PublicInfo> {
    VilResponse::ok(PublicInfo {
        service: "vil-auth-middleware-stack".into(),
        version: "0.1.0".into(),
        status: "operational".into(),
        auth_required_endpoints: vec![
            "GET /api/protected/data".into(),
            "GET /api/protected/tenant".into(),
        ],
    })
}

/// GET /tenant — extract tenant_id from JWT claims.
async fn tenant_info(headers: vil_server::axum::http::HeaderMap) -> HandlerResult<VilResponse<TenantInfo>> {
    let token = extract_bearer(&headers)?;
    let claims = decode_token(&token)?;

    Ok(VilResponse::ok(TenantInfo {
        tenant_id: claims.tenant_id,
        username: claims.sub,
        roles: claims.roles,
        issued_at: claims.iat,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state = Arc::new(AuthState::new(10)); // 10 requests per minute per tenant

    let admin_svc = ServiceProcess::new("admin")
        .endpoint(Method::POST, "/login", post(login_handler));

    let protected_svc = ServiceProcess::new("protected")
        .endpoint(Method::GET, "/data", get(protected_data))
        .endpoint(Method::GET, "/tenant", get(tenant_info))
        .state(state);

    let public_svc = ServiceProcess::new("public")
        .endpoint(Method::GET, "/info", get(public_info));

    VilApp::new("auth-middleware-stack")
        .port(8080)
        .observer(true)
        .service(admin_svc)
        .service(protected_svc)
        .service(public_svc)
        .run()
        .await;
}
