// 610 — Multi-Cloud Storage (VWFD)
// Business logic identical to standard:
//   POST /storage/upload, GET /storage/download/:id, GET /storage/list
//   Response: UploadResponse { status, asset: AssetMeta, provider_note }
//   AssetMeta { id, filename, content_type, size_bytes, provider, created_at }
use serde_json::{json, Value};

fn download_asset(input: &Value) -> Result<Value, String> {
    let path = input["path"].as_str().unwrap_or("");
    let id = path.split('/').last().unwrap_or("asset-001");
    Ok(json!({
        "asset": {
            "id": id,
            "filename": "report.pdf",
            "content_type": "application/pdf",
            "size_bytes": 245000,
            "provider": "s3",
            "created_at": "2024-01-15T10:00:00Z"
        },
        "content_base64": "JVBERi0xLjQK..."
    }))
}

fn list_assets(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "total": 3,
        "assets": [
            {"id": "asset-001", "filename": "report.pdf", "content_type": "application/pdf", "size_bytes": 245000, "provider": "s3", "created_at": "2024-01-15T10:00:00Z"},
            {"id": "asset-002", "filename": "photo.jpg", "content_type": "image/jpeg", "size_bytes": 1200000, "provider": "gcs", "created_at": "2024-01-14T08:30:00Z"},
            {"id": "asset-003", "filename": "data.csv", "content_type": "text/csv", "size_bytes": 89000, "provider": "azure", "created_at": "2024-01-13T15:45:00Z"}
        ],
        "providers_available": ["s3", "gcs", "azure"]
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/610-storage-multi-cloud/vwfd/workflows", 8080)
        .native("download_asset", download_asset)
        .native("list_assets", list_assets)
        .run().await;
}
