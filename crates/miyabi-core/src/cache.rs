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

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("test_value", Duration::from_secs(60));
        assert_eq!(entry.value, "test_value");
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_entry_expired() {
        let entry = CacheEntry::new("test_value", Duration::from_millis(0));
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[tokio::test]
    async fn test_ttl_cache_insert_with_custom_ttl() {
        let cache = TTLCache::new(Duration::from_secs(3600)); // default 1 hour

        // Insert with shorter custom TTL
        cache.insert_with_ttl("key1", "value1", Duration::from_millis(50)).await;
        assert_eq!(cache.get(&"key1").await, Some("value1"));

        sleep(Duration::from_millis(100)).await;
        assert_eq!(cache.get(&"key1").await, None);
    }

    #[tokio::test]
    async fn test_ttl_cache_remove() {
        let cache = TTLCache::new(Duration::from_secs(3600));

        cache.insert("key1", "value1").await;
        assert_eq!(cache.get(&"key1").await, Some("value1"));

        let removed = cache.remove(&"key1").await;
        assert_eq!(removed, Some("value1"));
        assert_eq!(cache.get(&"key1").await, None);
    }

    #[tokio::test]
    async fn test_ttl_cache_remove_nonexistent() {
        let cache: TTLCache<String, String> = TTLCache::new(Duration::from_secs(3600));
        let removed = cache.remove(&"nonexistent".to_string()).await;
        assert_eq!(removed, None);
    }

    #[tokio::test]
    async fn test_ttl_cache_stats() {
        let cache = TTLCache::new(Duration::from_millis(100));

        cache.insert("key1", "value1").await;
        cache.insert("key2", "value2").await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_ttl_cache_stats_with_expired() {
        let cache = TTLCache::new(Duration::from_millis(50));

        cache.insert("key1", "value1").await;
        cache.insert_with_ttl("key2", "value2", Duration::from_secs(3600)).await;

        sleep(Duration::from_millis(100)).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.expired_entries, 1);
        assert_eq!(stats.active_entries, 1);
    }

    #[tokio::test]
    async fn test_ttl_cache_clear() {
        let cache = TTLCache::new(Duration::from_secs(3600));

        cache.insert("key1", "value1").await;
        cache.insert("key2", "value2").await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 2);

        cache.clear().await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_ttl_cache_multiple_types() {
        let cache: TTLCache<i32, String> = TTLCache::new(Duration::from_secs(3600));

        cache.insert(1, "one".to_string()).await;
        cache.insert(2, "two".to_string()).await;

        assert_eq!(cache.get(&1).await, Some("one".to_string()));
        assert_eq!(cache.get(&2).await, Some("two".to_string()));
    }

    #[test]
    fn test_llm_cache_key_without_temperature() {
        let key1 = LLMCacheKey::new("prompt", "model", None);
        let key2 = LLMCacheKey::new("prompt", "model", None);
        let key3 = LLMCacheKey::new("prompt", "model", Some(0.5));

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert!(key1.temperature.is_none());
        assert!(key3.temperature.is_some());
    }

    #[test]
    fn test_llm_cache_key_different_models() {
        let key1 = LLMCacheKey::new("prompt", "claude-3-sonnet", Some(0.7));
        let key2 = LLMCacheKey::new("prompt", "claude-3-opus", Some(0.7));

        assert_ne!(key1, key2);
        assert_eq!(key1.model, "claude-3-sonnet");
        assert_eq!(key2.model, "claude-3-opus");
    }

    #[test]
    fn test_api_cache_key_creation() {
        let key1 = ApiCacheKey::new("/api/v1/messages", "{\"prompt\":\"test\"}");
        let key2 = ApiCacheKey::new("/api/v1/messages", "{\"prompt\":\"test\"}");
        let key3 = ApiCacheKey::new("/api/v1/messages", "{\"prompt\":\"different\"}");

        assert_eq!(key1.endpoint, "/api/v1/messages");
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_api_cache_key_different_endpoints() {
        let key1 = ApiCacheKey::new("/api/v1/messages", "{}");
        let key2 = ApiCacheKey::new("/api/v2/messages", "{}");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_create_llm_cache() {
        let cache = create_llm_cache();
        // Default TTL is 1 hour
        assert_eq!(cache.default_ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_create_api_cache() {
        let cache = create_api_cache();
        // Default TTL is 30 minutes
        assert_eq!(cache.default_ttl, Duration::from_secs(1800));
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            total_entries: 100,
            expired_entries: 10,
            active_entries: 90,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("10"));
        assert!(json.contains("90"));

        let deserialized: CacheStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_entries, 100);
    }

    #[tokio::test]
    async fn test_ttl_cache_concurrent_access() {
        let cache = Arc::new(TTLCache::new(Duration::from_secs(3600)));
        let cache_clone = cache.clone();

        let handle = tokio::spawn(async move {
            for i in 0..10 {
                cache_clone.insert(i, i * 2).await;
            }
        });

        handle.await.unwrap();

        for i in 0..10 {
            assert_eq!(cache.get(&i).await, Some(i * 2));
        }
    }

    #[tokio::test]
    async fn test_llm_cache_full_workflow() {
        let cache = create_llm_cache();
        let key = LLMCacheKey::new("Hello, world!", "claude-3-sonnet", Some(0.7));

        // Miss
        assert!(cache.get(&key).await.is_none());

        // Insert
        cache.insert(key.clone(), "Response from LLM".to_string()).await;

        // Hit
        let cached = cache.get(&key).await;
        assert_eq!(cached, Some("Response from LLM".to_string()));

        // Different key misses
        let different_key = LLMCacheKey::new("Different prompt", "claude-3-sonnet", Some(0.7));
        assert!(cache.get(&different_key).await.is_none());
    }
}
