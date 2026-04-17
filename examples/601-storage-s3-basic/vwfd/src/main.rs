// 601 — S3 Storage Upload (VWFD)
use serde_json::{json, Value};

fn s3_upload(input: &Value) -> Result<Value, String> {
    let body = &input["body"];
    Ok(json!({
        "status": "uploaded",
        "bucket": body["bucket"].as_str().unwrap_or("default"),
        "key": body["key"].as_str().unwrap_or("unnamed"),
        "etag": "\"d41d8cd98f00b204e9800998ecf8427e\"",
        "content_type": body["content_type"].as_str().unwrap_or("application/octet-stream")
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/601-storage-s3-basic/vwfd/workflows", 3241)
        .native("s3_upload", s3_upload)
        .run().await;
}
