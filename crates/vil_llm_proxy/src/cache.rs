//! Semantic response cache for LLM proxy.
//!
//! Uses FNV hash of serialized messages as key. DashMap-backed with configurable
//! TTL and max entries with LRU eviction.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use vil_llm::{ChatMessage, ChatResponse, Usage};

/// Cached response entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<CachedUsage>,
    #[serde(skip)]
    pub cached_at: Option<Instant>,
}

/// Serializable usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<&Usage> for CachedUsage {
    fn from(u: &Usage) -> Self {
        Self {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }
    }
}

impl From<&ChatResponse> for CachedResponse {
    fn from(r: &ChatResponse) -> Self {
        Self {
            content: r.content.clone(),
            model: r.model.clone(),
            usage: r.usage.as_ref().map(CachedUsage::from),
            cached_at: Some(Instant::now()),
        }
    }
}

impl CachedResponse {
    /// Convert back to a ChatResponse.
    pub fn to_chat_response(&self) -> ChatResponse {
        ChatResponse {
            content: self.content.clone(),
            model: self.model.clone(),
            tool_calls: None,
            usage: self.usage.as_ref().map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: Some("cache_hit".to_string()),
        }
    }
}

/// Internal cache entry with timestamp for TTL/LRU.
struct CacheEntry {
    response: CachedResponse,
    inserted_at: Instant,
    last_accessed: Instant,
}

/// Semantic response cache.
pub struct ResponseCache {
    entries: DashMap<u64, CacheEntry>,
    ttl: Duration,
    max_entries: usize,
}

impl ResponseCache {
    /// Create a new cache with default settings (5 min TTL, 1000 max entries).
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            ttl: Duration::from_secs(300),
            max_entries: 1000,
        }
    }

    /// Create a cache with custom TTL and max entries.
    pub fn with_config(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
            max_entries,
        }
    }

    /// Compute FNV-style hash of messages.
    fn hash_messages(messages: &[ChatMessage]) -> u64 {
        let serialized = serde_json::to_string(messages).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        serialized.hash(&mut hasher);
        hasher.finish()
    }

    /// Look up a cached response.
    pub fn get(&self, messages: &[ChatMessage]) -> Option<CachedResponse> {
        let key = Self::hash_messages(messages);
        let now = Instant::now();

        if let Some(mut entry) = self.entries.get_mut(&key) {
            // Check TTL
            if now.duration_since(entry.inserted_at) > self.ttl {
                drop(entry);
                self.entries.remove(&key);
                return None;
            }
            entry.last_accessed = now;
            Some(entry.response.clone())
        } else {
            None
        }
    }

    /// Store a response in the cache.
    pub fn put(&self, messages: &[ChatMessage], response: &ChatResponse) {
        let key = Self::hash_messages(messages);
        let now = Instant::now();

        // Evict if at capacity
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        self.entries.insert(key, CacheEntry {
            response: CachedResponse::from(response),
            inserted_at: now,
            last_accessed: now,
        });
    }

    /// Evict the least-recently-used entry.
    fn evict_lru(&self) {
        let mut oldest_key = None;
        let mut oldest_time = Instant::now();

        for entry in self.entries.iter() {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(*entry.key());
            }
        }

        if let Some(key) = oldest_key {
            self.entries.remove(&key);
        }
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.clear();
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_llm::ChatMessage;

    fn sample_messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Hello!"),
        ]
    }

    fn sample_response() -> ChatResponse {
        ChatResponse {
            content: "Hi there!".to_string(),
            model: "gpt-4".to_string(),
            tool_calls: None,
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            finish_reason: Some("stop".to_string()),
        }
    }

    #[test]
    fn test_put_and_get() {
        let cache = ResponseCache::new();
        let msgs = sample_messages();
        let resp = sample_response();

        cache.put(&msgs, &resp);

        let cached = cache.get(&msgs).unwrap();
        assert_eq!(cached.content, "Hi there!");
        assert_eq!(cached.model, "gpt-4");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResponseCache::new();
        let msgs = vec![ChatMessage::user("unknown query")];
        assert!(cache.get(&msgs).is_none());
    }

    #[test]
    fn test_ttl_expiry() {
        let cache = ResponseCache::with_config(Duration::from_millis(1), 100);
        let msgs = sample_messages();
        let resp = sample_response();

        cache.put(&msgs, &resp);

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(10));

        assert!(cache.get(&msgs).is_none());
    }

    #[test]
    fn test_max_entries_eviction() {
        let cache = ResponseCache::with_config(Duration::from_secs(60), 2);

        let m1 = vec![ChatMessage::user("query 1")];
        let m2 = vec![ChatMessage::user("query 2")];
        let m3 = vec![ChatMessage::user("query 3")];
        let resp = sample_response();

        cache.put(&m1, &resp);
        cache.put(&m2, &resp);
        assert_eq!(cache.len(), 2);

        cache.put(&m3, &resp);
        // Should still be at max (one was evicted)
        assert!(cache.len() <= 2);
    }

    #[test]
    fn test_cached_response_to_chat_response() {
        let cache = ResponseCache::new();
        let msgs = sample_messages();
        let resp = sample_response();

        cache.put(&msgs, &resp);
        let cached = cache.get(&msgs).unwrap();
        let converted = cached.to_chat_response();

        assert_eq!(converted.content, resp.content);
        assert_eq!(converted.model, resp.model);
        assert_eq!(converted.finish_reason, Some("cache_hit".to_string()));
    }
}
