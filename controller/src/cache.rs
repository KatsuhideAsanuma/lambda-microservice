// TEMPORARILY DISABLED - Redis functionality disabled due to compatibility issues
// Original Redis implementation moved to cache_redis_disabled.rs

use crate::error::{Error, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
pub trait RedisPoolTrait: Send + Sync + 'static {
    async fn get_value_raw(&self, key: &str) -> Result<Option<String>>;
    async fn set_ex_raw(&self, key: &str, value: &str, expiry_seconds: u64) -> Result<()>;
    async fn del(&self, key: &str) -> Result<()>;
}

impl dyn RedisPoolTrait {
    pub async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>> {
        let raw = self.get_value_raw(key).await?;
        match raw {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn set_ex<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        expiry_seconds: u64,
    ) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        self.set_ex_raw(key, &serialized, expiry_seconds).await
    }
}

// In-memory cache implementation as temporary replacement for Redis
#[derive(Clone)]
pub struct InMemoryCache {
    storage: Arc<Mutex<HashMap<String, (String, Option<u64>)>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RedisPoolTrait for InMemoryCache {
    async fn get_value_raw(&self, key: &str) -> Result<Option<String>> {
        let storage = self.storage.lock().await;
        Ok(storage.get(key).map(|(value, _)| value.clone()))
    }

    async fn set_ex_raw(&self, key: &str, value: &str, _expiry_seconds: u64) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.insert(key.to_string(), (value.to_string(), Some(_expiry_seconds)));
        Ok(())
    }

    async fn del(&self, key: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.remove(key);
        Ok(())
    }
}

// Type alias for compatibility
pub type RedisPool = InMemoryCache;

#[derive(Clone)]
pub struct RedisClient<P: RedisPoolTrait + Clone> {
    pool: P,
    ttl_seconds: u64,
}

impl<P: RedisPoolTrait + Clone> RedisClient<P> {
    pub fn new_with_pool(pool: P, ttl_seconds: u64) -> Self {
        Self { pool, ttl_seconds }
    }

    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    pub async fn get_wasm_module(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let raw = self.pool.get_value_raw(key).await?;
        match raw {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn cache_wasm_module(&self, key: &str, wasm_bytes: &[u8]) -> Result<()> {
        let serialized = serde_json::to_string(&wasm_bytes.to_vec())?;
        self.pool
            .set_ex_raw(key, &serialized, self.ttl_seconds)
            .await
    }

    pub async fn cache_session(&self, session: &crate::session::Session) -> Result<()> {
        let key = format!("session:{}", session.request_id);
        let serialized = serde_json::to_string(&session)?;
        self.pool
            .set_ex_raw(&key, &serialized, self.ttl_seconds)
            .await
    }
}

impl RedisClient<InMemoryCache> {
    pub async fn new(_redis_url: &str) -> Result<Self> {
        // Ignore redis_url and use in-memory cache instead
        let pool = InMemoryCache::new();
        Ok(Self {
            pool,
            ttl_seconds: 3600, // Default to 1 hour TTL
        })
    }
}

impl InMemoryCache {
    pub async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>> {
        let raw = self.get_value_raw(key).await?;
        match raw {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn set_ex<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        expiry_seconds: u64,
    ) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        self.set_ex_raw(key, &serialized, expiry_seconds).await
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let storage = self.storage.lock().await;
        Ok(storage.contains_key(key))
    }

    pub async fn set_nx_ex<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        expiry_seconds: u64,
    ) -> Result<bool> {
        let mut storage = self.storage.lock().await;
        if storage.contains_key(key) {
            Ok(false)
        } else {
            let serialized = serde_json::to_string(value)?;
            storage.insert(key.to_string(), (serialized, Some(expiry_seconds)));
            Ok(true)
        }
    }

    pub async fn expire(&self, key: &str, expiry_seconds: u64) -> Result<bool> {
        let mut storage = self.storage.lock().await;
        if let Some((_value, expiry)) = storage.get_mut(key) {
            *expiry = Some(expiry_seconds);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(any(test, feature = "test-integration"))]
#[cfg_attr(test, path = "cache/tests.rs")]
pub mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u64,
        name: String,
        value: f64,
    }

    #[derive(Clone)]
    pub struct MockRedisPool {
        get_result: Arc<Mutex<Result<Option<String>>>>,
        set_ex_result: Arc<Mutex<Result<()>>>,
        del_result: Arc<Mutex<Result<()>>>,
        exists_result: Arc<Mutex<Result<bool>>>,
        set_nx_ex_result: Arc<Mutex<Result<bool>>>,
        expire_result: Arc<Mutex<Result<bool>>>,
    }

    impl MockRedisPool {
        pub fn new() -> Self {
            Self {
                get_result: Arc::new(Mutex::new(Ok(None))),
                set_ex_result: Arc::new(Mutex::new(Ok(()))),
                del_result: Arc::new(Mutex::new(Ok(()))),
                exists_result: Arc::new(Mutex::new(Ok(false))),
                set_nx_ex_result: Arc::new(Mutex::new(Ok(true))),
                expire_result: Arc::new(Mutex::new(Ok(true))),
            }
        }

        pub fn with_get_result(mut self, result: Result<Option<String>>) -> Self {
            self.get_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_set_ex_result(mut self, result: Result<()>) -> Self {
            self.set_ex_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_del_result(mut self, result: Result<()>) -> Self {
            self.del_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_exists_result(mut self, result: Result<bool>) -> Self {
            self.exists_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_set_nx_ex_result(mut self, result: Result<bool>) -> Self {
            self.set_nx_ex_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_expire_result(mut self, result: Result<bool>) -> Self {
            self.expire_result = Arc::new(Mutex::new(result));
            self
        }

        pub async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(
            &self,
            _key: &str,
        ) -> Result<Option<T>> {
            let result = self.get_result.lock().await.clone()?;
            match result {
                Some(value) => Ok(Some(serde_json::from_str(&value)?)),
                None => Ok(None),
            }
        }

        pub async fn set_ex<T: Serialize + Send + Sync>(
            &self,
            _key: &str,
            _value: &T,
            _expiry_seconds: u64,
        ) -> Result<()> {
            self.set_ex_result.lock().await.clone()
        }

        pub async fn del(&self, _key: &str) -> Result<()> {
            self.del_result.lock().await.clone()
        }

        pub async fn exists(&self, _key: &str) -> Result<bool> {
            self.exists_result.lock().await.clone()
        }

        pub async fn set_nx_ex<T: Serialize>(
            &self,
            _key: &str,
            _value: &T,
            _expiry_seconds: u64,
        ) -> Result<bool> {
            self.set_nx_ex_result.lock().await.clone()
        }

        pub async fn expire(&self, _key: &str, _expiry_seconds: u64) -> Result<bool> {
            self.expire_result.lock().await.clone()
        }
    }

    #[async_trait]
    impl RedisPoolTrait for MockRedisPool {
        async fn get_value_raw(&self, _key: &str) -> Result<Option<String>> {
            self.get_result.lock().await.clone()
        }

        async fn set_ex_raw(&self, _key: &str, _value: &str, _expiry_seconds: u64) -> Result<()> {
            self.set_ex_result.lock().await.clone()
        }

        async fn del(&self, _key: &str) -> Result<()> {
            self.del_result.lock().await.clone()
        }
    }

    #[tokio::test]
    async fn test_in_memory_cache_basic_operations() {
        let cache = InMemoryCache::new();

        let test_data = TestData {
            id: 1,
            name: "Test".to_string(),
            value: 42.5,
        };

        // Test set and get
        let result = cache.set_ex("test_key", &test_data, 60).await;
        assert!(result.is_ok());

        let retrieved: Result<Option<TestData>> = cache.get_value("test_key").await;
        assert!(retrieved.is_ok());

        let data = retrieved.unwrap();
        assert!(data.is_some());
        assert_eq!(data.unwrap(), test_data);

        // Test exists
        let exists = cache.exists("test_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());

        // Test delete
        let result = cache.del("test_key").await;
        assert!(result.is_ok());

        let exists = cache.exists("test_key").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn test_in_memory_cache_set_nx_ex() {
        let cache = InMemoryCache::new();

        let test_data1 = TestData {
            id: 1,
            name: "First".to_string(),
            value: 10.0,
        };

        let test_data2 = TestData {
            id: 2,
            name: "Second".to_string(),
            value: 20.0,
        };

        // First set should succeed
        let result = cache.set_nx_ex("nx_key", &test_data1, 60).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Second set should fail (key exists)
        let result = cache.set_nx_ex("nx_key", &test_data2, 60).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Verify original value is still there
        let retrieved: Result<Option<TestData>> = cache.get_value("nx_key").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().unwrap(), test_data1);
    }
}
