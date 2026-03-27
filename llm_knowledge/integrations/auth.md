# Auth Integration

VIL provides JWT, rate limiting, RBAC, and CORS as built-in middleware.

## JWT Authentication

```rust
use vil_server::auth::{JwtConfig, JwtClaims};

VilApp::new("secure-api")
    .port(8080)
    .jwt(JwtConfig::new()
        .secret("${ENV:VIL_JWT_SECRET}")
        .issuer("my-app")
        .expiry(Duration::from_secs(3600)))
    .service(api_service)
    .run()
    .await;
```

### Extract Claims in Handler

```rust
#[vil_handler(shm)]
async fn protected(claims: JwtClaims, ctx: ServiceCtx) -> VilResponse<Profile> {
    let user_id = claims.sub;  // Subject from JWT
    let roles = claims.roles;  // Custom claims
    let db = ctx.state::<VilDbPool>();
    let profile = fetch_profile(db, &user_id).await?;
    VilResponse::ok(profile)
}
```

## Rate Limiting

```rust
use vil_server::auth::RateLimitConfig;

VilApp::new("rate-limited")
    .port(8080)
    .rate_limit(RateLimitConfig::new()
        .requests_per_minute(100)
        .burst(20)
        .key_extractor(KeyExtractor::IpAddress))
    .service(api_service)
    .run()
    .await;
```

## RBAC (Role-Based Access Control)

```rust
use vil_server::auth::{RbacConfig, Role, Permission};

let rbac = RbacConfig::new()
    .role(Role::new("admin").permit(Permission::All))
    .role(Role::new("user").permit(Permission::Read))
    .role(Role::new("editor").permit(Permission::ReadWrite));

VilApp::new("rbac-api")
    .port(8080)
    .rbac(rbac)
    .service(api_service)
    .run()
    .await;
```

### Require Role in Handler

```rust
#[vil_handler(shm)]
async fn admin_only(claims: JwtClaims) -> VilResponse<AdminData> {
    claims.require_role("admin")?;  // Returns 403 if missing
    VilResponse::ok(get_admin_data())
}
```

## CORS

```rust
use vil_server::auth::CorsConfig;

VilApp::new("cors-api")
    .port(8080)
    .cors(CorsConfig::new()
        .allowed_origins(vec!["https://myapp.com"])
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec!["Content-Type", "Authorization"])
        .max_age(3600))
    .service(api_service)
    .run()
    .await;

// Or permissive for development:
.cors(CorsConfig::permissive())
```

## API Key

```rust
use vil_server::auth::ApiKeyConfig;

VilApp::new("api-key-service")
    .port(8080)
    .api_key(ApiKeyConfig::new()
        .header("X-Api-Key")
        .keys(vec!["key-abc-123", "key-def-456"]))
    .service(api_service)
    .run()
    .await;
```

## Combining Auth Layers

```rust
VilApp::new("production")
    .port(8080)
    .cors(CorsConfig::new().allowed_origins(vec!["https://app.com"]))
    .jwt(JwtConfig::new().secret("${ENV:JWT_SECRET}"))
    .rbac(rbac_config)
    .rate_limit(RateLimitConfig::new().requests_per_minute(500))
    .service(api_service)
    .run()
    .await;
```

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
