// ╔════════════════════════════════════════════════════════════╗
// ║  610 — Media Platform: Multi-Cloud Asset Storage           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Media Platform — Multi-Cloud Asset Storage       ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: ServiceCtx, ShmSlice, VilResponse, in-memory    ║
// ║            mock store (S3/GCS/Azure pattern)                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Upload/download media assets with provider       ║
// ║  selection. In production, swap the MockStore for           ║
// ║  vil_storage_s3::S3Client, vil_storage_gcs, or              ║
// ║  vil_storage_azure. This demo uses an in-memory HashMap     ║
// ║  so it runs without cloud credentials.                      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Production pattern with real cloud storage:
//
//   use vil_storage_s3::{S3Client, S3Config};
//   let s3 = S3Client::new(S3Config {
//       endpoint: Some("http://localhost:9000".into()),
//       region: "us-east-1".into(),
//       access_key: Some("minioadmin".into()),
//       secret_key: Some("minioadmin".into()),
//       bucket: "media-assets".into(),
//       path_style: true,
//   }).await.expect("S3 connect");
//
//   // GCS and Azure follow the same pattern via vil_storage_gcs / vil_storage_azure
//
// Run:   cargo run -p vil-storage-multi-cloud
// Test:
//   curl -X POST http://localhost:8080/api/storage/upload \
//     -H 'Content-Type: application/json' \
//     -d '{"filename":"logo.png","provider":"s3","content_base64":"aGVsbG8gd29ybGQ=","content_type":"image/png"}'
//
//   curl http://localhost:8080/api/storage/list
//
//   curl http://localhost:8080/api/storage/download/<id>

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use vil_server::prelude::*;

// ── Storage Provider Enum ────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StorageProvider {
    S3,
    Gcs,
    Azure,
}

impl std::fmt::Display for StorageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Azure => write!(f, "azure"),
        }
    }
}

// ── Asset Metadata ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct AssetMeta {
    id: String,
    filename: String,
    content_type: String,
    size_bytes: usize,
    provider: StorageProvider,
    created_at: String,
}

// ── In-Memory Mock Store ─────────────────────────────────────────────────
// In production, replace with real S3/GCS/Azure clients. The trait-based
// pattern allows swapping implementations without changing handler code.

struct StoredAsset {
    meta: AssetMeta,
    data: Vec<u8>,
}

struct MultiCloudStore {
    assets: RwLock<HashMap<String, StoredAsset>>,
}

impl MultiCloudStore {
    fn new() -> Self {
        Self {
            assets: RwLock::new(HashMap::new()),
        }
    }

    async fn upload(
        &self,
        id: String,
        filename: String,
        content_type: String,
        provider: StorageProvider,
        data: Vec<u8>,
    ) -> AssetMeta {
        let meta = AssetMeta {
            id: id.clone(),
            filename,
            content_type,
            size_bytes: data.len(),
            provider,
            created_at: "2026-04-05T10:00:00Z".into(),
        };
        let stored = StoredAsset {
            meta: meta.clone(),
            data,
        };
        self.assets.write().await.insert(id, stored);
        meta
    }

    async fn download(&self, id: &str) -> Option<(AssetMeta, Vec<u8>)> {
        let guard = self.assets.read().await;
        guard.get(id).map(|s| (s.meta.clone(), s.data.clone()))
    }

    async fn list(&self) -> Vec<AssetMeta> {
        let guard = self.assets.read().await;
        guard.values().map(|s| s.meta.clone()).collect()
    }
}

struct AppState {
    store: MultiCloudStore,
}

// ── Request / Response Models ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct UploadRequest {
    filename: String,
    provider: StorageProvider,
    /// Base64-encoded file content (for JSON-based demo upload).
    content_base64: String,
    #[serde(default = "default_content_type")]
    content_type: String,
}

fn default_content_type() -> String {
    "application/octet-stream".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct UploadResponse {
    status: String,
    asset: AssetMeta,
    provider_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct DownloadResponse {
    asset: AssetMeta,
    content_base64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ListResponse {
    total: usize,
    assets: Vec<AssetMeta>,
    providers_available: Vec<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /api/storage/upload — upload asset to selected cloud provider.
async fn upload(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<UploadResponse>> {
    let req: UploadRequest = body.json()
        .map_err(|_| VilError::bad_request(
            "invalid JSON — expected {\"filename\", \"provider\", \"content_base64\"}"
        ))?;

    // Decode base64 content
    let data = base64_decode(&req.content_base64)
        .map_err(|e| VilError::bad_request(format!("invalid base64: {}", e)))?;

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let id = uuid::Uuid::new_v4().to_string();

    let provider_note = match req.provider {
        StorageProvider::S3 => {
            "Demo mode: in-memory store. Production: use vil_storage_s3::S3Client \
             with S3Config { endpoint, region, access_key, secret_key, bucket }."
        }
        StorageProvider::Gcs => {
            "Demo mode: in-memory store. Production: use vil_storage_gcs \
             with GCS service account credentials and bucket name."
        }
        StorageProvider::Azure => {
            "Demo mode: in-memory store. Production: use vil_storage_azure \
             with Azure Blob Storage connection string and container."
        }
    };

    let meta = state.store.upload(
        id, req.filename, req.content_type, req.provider, data,
    ).await;

    Ok(VilResponse::ok(UploadResponse {
        status: "uploaded".into(),
        asset: meta,
        provider_note: provider_note.into(),
    }))
}

/// GET /api/storage/download/:id — download asset by ID.
async fn download(
    ctx: ServiceCtx,
    Path(id): Path<String>,
) -> HandlerResult<VilResponse<DownloadResponse>> {

    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let (meta, data) = state.store.download(&id).await
        .ok_or_else(|| VilError::not_found(format!("asset {} not found", id)))?;

    Ok(VilResponse::ok(DownloadResponse {
        asset: meta,
        content_base64: base64_encode(&data),
    }))
}

/// GET /api/storage/list — list all stored assets across providers.
async fn list_assets(
    ctx: ServiceCtx,
) -> HandlerResult<VilResponse<ListResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let assets = state.store.list().await;

    Ok(VilResponse::ok(ListResponse {
        total: assets.len(),
        assets,
        providers_available: vec!["s3".into(), "gcs".into(), "azure".into()],
    }))
}

// ── Base64 helpers (no extra dependency) ──────────────────────────────────

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    // Simple base64 decoder for demo purposes
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.trim().replace('\n', "").replace('\r', "");
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for c in input.bytes() {
        if c == b'=' { break; }
        let val = TABLE.iter().position(|&t| t == c)
            .ok_or_else(|| format!("invalid base64 char: {}", c as char))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        store: MultiCloudStore::new(),
    });

    let svc = ServiceProcess::new("storage")
        .prefix("/api")
        .endpoint(Method::POST, "/storage/upload", post(upload))
        .endpoint(Method::GET, "/storage/download/:id", get(download))
        .endpoint(Method::GET, "/storage/list", get(list_assets))
        .state(state);

    VilApp::new("multi-cloud-storage")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
