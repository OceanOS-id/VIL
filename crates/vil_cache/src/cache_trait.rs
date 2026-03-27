use std::time::Duration;

/// Cache trait — 1 vtable call per operation.
#[async_trait::async_trait]
pub trait VilCache: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>);
    async fn del(&self, key: &str) -> bool;
    async fn exists(&self, key: &str) -> bool;

    /// JSON convenience.
    async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let bytes = self.get(key).await?;
        serde_json::from_slice(&bytes).ok()
    }

    async fn set_json<T: serde::Serialize + Sync>(&self, key: &str, value: &T, ttl: Option<Duration>) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            self.set(key, &bytes, ttl).await;
        }
    }
}
