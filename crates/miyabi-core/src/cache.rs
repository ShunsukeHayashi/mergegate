//! Caching utilities for Miyabi
//!
//! Provides in-memory caching with TTL support for LLM responses and other expensive operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache entry with TTL support
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// Cached value
    pub value: T,
    /// Expiration timestamp
    pub expires_at: Instant,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry with the given TTL
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    /// Check if the cache entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Thread-safe TTL cache
#[derive(Debug)]
pub struct TTLCache<K, V> {
    inner: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Duration,
}

impl<K, V> TTLCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new TTL cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().await;

        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                cache.remove(key);
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    /// Insert a value into the cache with default TTL
    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }

    /// Insert a value into the cache with custom TTL
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut cache = self.inner.write().await;
        cache.insert(key, CacheEntry::new(value, ttl));
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write().await;
        cache.remove(key).map(|entry| entry.value)
    }

    /// Clear all expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.inner.write().await;
        let initial_size = cache.len();
        cache.retain(|_, entry| !entry.is_expired());
        initial_size - cache.len()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.inner.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| entry.is_expired()).count();

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
        }
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut cache = self.inner.write().await;
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total entries
    pub total_entries: usize,
    /// Expired entries
    pub expired_entries: usize,
    /// Active entries
    pub active_entries: usize,
}

/// LLM response cache key
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LLMCacheKey {
    /// Prompt hash
    pub prompt_hash: String,
    /// Model name
    pub model: String,
    /// Temperature parameter (as bits to avoid f32 Hash/Eq issues)
    pub temperature: Option<u32>,
}

impl LLMCacheKey {
    pub fn new(prompt: &str, model: &str, temperature: Option<f32>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        model.hash(&mut hasher);
        if let Some(temp) = temperature {
            temp.to_bits().hash(&mut hasher);
        }

        Self {
            prompt_hash: format!("{:x}", hasher.finish()),
            model: model.to_string(),
            temperature: temperature.map(|t| t.to_bits()),
        }
    }
}

/// LLM response cache
pub type LLMCache = TTLCache<LLMCacheKey, String>;

/// Create a new LLM cache with 1 hour TTL
pub fn create_llm_cache() -> LLMCache {
    TTLCache::new(Duration::from_secs(3600))
}

/// API response cache key
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ApiCacheKey {
    /// Endpoint
    pub endpoint: String,
    /// Request hash
    pub request_hash: String,
}

impl ApiCacheKey {
    pub fn new(endpoint: &str, request_body: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        request_body.hash(&mut hasher);

        Self {
            endpoint: endpoint.to_string(),
            request_hash: format!("{:x}", hasher.finish()),
        }
    }
}

/// API response cache
pub type ApiCache = TTLCache<ApiCacheKey, String>;

/// Create a new API cache with 30 minutes TTL
pub fn create_api_cache() -> ApiCache {
    TTLCache::new(Duration::from_secs(1800))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_ttl_cache_basic_operations() {
        let cache = TTLCache::new(Duration::from_millis(100));

        cache.insert("key1", "value1").await;
        assert_eq!(cache.get(&"key1").await, Some("value1"));

        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get(&"key1").await, None);
    }

    #[tokio::test]
    async fn test_ttl_cache_cleanup() {
        let cache = TTLCache::new(Duration::from_millis(50));

        cache.insert("key1", "value1").await;
        cache.insert("key2", "value2").await;

        sleep(Duration::from_millis(100)).await;

        let cleaned = cache.cleanup_expired().await;
        assert_eq!(cleaned, 2);

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_llm_cache_key() {
        let key1 = LLMCacheKey::new("prompt1", "model1", Some(0.7));
        let key2 = LLMCacheKey::new("prompt1", "model1", Some(0.7));
        let key3 = LLMCacheKey::new("prompt2", "model1", Some(0.7));

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
