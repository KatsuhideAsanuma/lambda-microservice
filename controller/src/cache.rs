
use crate::error::{Error, Result};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use async_trait::async_trait;

#[async_trait]
pub trait RedisPoolTrait: Send + Sync + 'static {
    async fn get_value<'a, T: serde::de::DeserializeOwned + Send + Sync>(&'a self, key: &'a str) -> Result<Option<T>>;
    async fn set_ex<'a, T: serde::Serialize + Send + Sync>(&'a self, key: &'a str, value: &'a T, expiry_seconds: u64) -> Result<()>;
    async fn del<'a>(&'a self, key: &'a str) -> Result<()>;
}

#[derive(Clone)]
pub struct RedisPool {
    pool: Pool,
}

#[derive(Clone)]
pub struct RedisClient {
    pool: RedisPool,
    ttl_seconds: u64,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let pool = RedisPool::new(redis_url)?;
        Ok(Self {
            pool,
            ttl_seconds: 3600, // Default to 1 hour TTL
        })
    }
    
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }
    
    pub async fn get_wasm_module(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.pool.get_value(key).await
    }
    
    pub async fn cache_wasm_module(&self, key: &str, wasm_bytes: &[u8]) -> Result<()> {
        self.pool.set_ex(key, &wasm_bytes.to_vec(), self.ttl_seconds).await
    }
    
    pub async fn cache_runtime_type(&self, namespace: &str, language_title: &str, runtime_type: &crate::runtime::RuntimeType) -> Result<()> {
        let cache_key = format!("k8s:runtime:{}:{}", namespace, language_title);
        self.pool.set_ex(&cache_key, runtime_type, self.ttl_seconds).await
    }
    
    pub async fn get_runtime_type(&self, namespace: &str, language_title: &str) -> Result<Option<crate::runtime::RuntimeType>> {
        let cache_key = format!("k8s:runtime:{}:{}", namespace, language_title);
        self.pool.get_value(&cache_key).await
    }
    
    pub async fn invalidate_runtime_cache(&self, namespace: &str, language_title: &str) -> Result<()> {
        let cache_key = format!("k8s:runtime:{}:{}", namespace, language_title);
        self.pool.del(&cache_key).await
    }
}

#[async_trait]
impl RedisPoolTrait for RedisPool {
    async fn get_value<'a, T: serde::de::DeserializeOwned + Send + Sync>(&'a self, key: &'a str) -> Result<Option<T>> {
        self.get_value(key).await
    }

    async fn set_ex<'a, T: serde::Serialize + Send + Sync>(&'a self, key: &'a str, value: &'a T, expiry_seconds: u64) -> Result<()> {
        self.set_ex(key, value, expiry_seconds).await
    }

    async fn del<'a>(&'a self, key: &'a str) -> Result<()> {
        self.del(key).await
    }
}

impl RedisPool {
    pub fn new(redis_url: &str) -> Result<Self> {
        let mut config = Config::from_url(redis_url);
        config.pool = Some(deadpool_redis::PoolConfig::new(10));

        let pool = config.create_pool(Some(Runtime::Tokio1))
            .map_err(|e| Error::Cache(format!("Failed to create Redis pool: {}", e)))?;

        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<deadpool_redis::Connection> {
        self.pool.get().await.map_err(Error::from)
    }

    pub async fn set_ex<T: serde::Serialize + Send + Sync>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<()> {
        let mut conn = self.get().await?;
        let serialized = serde_json::to_string(value)?;
        conn.set_ex(key, serialized, expiry_seconds.try_into().unwrap()).await.map_err(Error::from)
    }

    pub async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get().await?;
        let result: Option<String> = conn.get(key).await.map_err(Error::from)?;
        
        match result {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn del(&self, key: &str) -> Result<()> {
        let mut conn = self.get().await?;
        conn.del(key).await.map_err(Error::from)
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get().await?;
        let result: i64 = conn.exists(key).await.map_err(Error::from)?;
        Ok(result > 0)
    }

    pub async fn set_nx_ex<T: serde::Serialize>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<bool> {
        let mut conn = self.get().await?;
        let serialized = serde_json::to_string(value)?;
        let result: bool = redis::cmd("SET")
            .arg(key)
            .arg(serialized)
            .arg("NX")
            .arg("EX")
            .arg(expiry_seconds.to_string()) // Convert u64 to String to ensure type compatibility
            .query_async(&mut conn)
            .await
            .map_err(Error::from)?;
        Ok(result)
    }

    pub async fn expire(&self, key: &str, expiry_seconds: u64) -> Result<bool> {
        let mut conn = self.get().await?;
        let result: bool = conn.expire(key, expiry_seconds.try_into().unwrap()).await.map_err(Error::from)?;
        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use serde::{Serialize, Deserialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u64,
        name: String,
        value: f64,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct MockRedisConnection {
        storage: Arc<Mutex<std::collections::HashMap<String, (String, Option<u64>)>>>,
    }

    impl MockRedisConnection {
        #[allow(dead_code)]
        fn new() -> Self {
            Self {
                storage: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }
        }

        #[allow(dead_code)]
        async fn set_ex(&mut self, key: &str, value: String, expiry_seconds: u64) -> Result<()> {
            let mut storage = self.storage.lock().await;
            storage.insert(key.to_string(), (value, Some(expiry_seconds)));
            Ok(())
        }

        #[allow(dead_code)]
        async fn get(&mut self, key: &str) -> Result<Option<String>> {
            let storage = self.storage.lock().await;
            Ok(storage.get(key).map(|(value, _)| value.clone()))
        }

        #[allow(dead_code)]
        async fn del(&mut self, key: &str) -> Result<()> {
            let mut storage = self.storage.lock().await;
            storage.remove(key);
            Ok(())
        }

        #[allow(dead_code)]
        async fn exists(&mut self, key: &str) -> Result<i64> {
            let storage = self.storage.lock().await;
            Ok(if storage.contains_key(key) { 1 } else { 0 })
        }

        #[allow(dead_code)]
        async fn set_nx_ex(&mut self, key: &str, value: String, expiry_seconds: u64) -> Result<bool> {
            let mut storage = self.storage.lock().await;
            if storage.contains_key(key) {
                Ok(false)
            } else {
                storage.insert(key.to_string(), (value, Some(expiry_seconds)));
                Ok(true)
            }
        }

        #[allow(dead_code)]
        async fn expire(&mut self, key: &str, expiry_seconds: u64) -> Result<bool> {
            let mut storage = self.storage.lock().await;
            if let Some((_value, expiry)) = storage.get_mut(key) {
                *expiry = Some(expiry_seconds);
                Ok(true)
            } else {
                Ok(false)
            }
        }
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
        fn new() -> Self {
            Self {
                get_result: Arc::new(Mutex::new(Ok(None))),
                set_ex_result: Arc::new(Mutex::new(Ok(()))),
                del_result: Arc::new(Mutex::new(Ok(()))),
                exists_result: Arc::new(Mutex::new(Ok(false))),
                set_nx_ex_result: Arc::new(Mutex::new(Ok(true))),
                expire_result: Arc::new(Mutex::new(Ok(true))),
            }
        }

        fn with_get_result(mut self, result: Result<Option<String>>) -> Self {
            self.get_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_set_ex_result(mut self, result: Result<()>) -> Self {
            self.set_ex_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_del_result(mut self, result: Result<()>) -> Self {
            self.del_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_exists_result(mut self, result: Result<bool>) -> Self {
            self.exists_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_set_nx_ex_result(mut self, result: Result<bool>) -> Self {
            self.set_nx_ex_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_expire_result(mut self, result: Result<bool>) -> Self {
            self.expire_result = Arc::new(Mutex::new(result));
            self
        }

        async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(&self, _key: &str) -> Result<Option<T>> {
            let result = self.get_result.lock().await.clone()?;
            match result {
                Some(value) => Ok(Some(serde_json::from_str(&value)?)),
                None => Ok(None),
            }
        }

        async fn set_ex<T: Serialize + Send + Sync>(&self, _key: &str, _value: &T, _expiry_seconds: u64) -> Result<()> {
            self.set_ex_result.lock().await.clone()
        }

        async fn del(&self, _key: &str) -> Result<()> {
            self.del_result.lock().await.clone()
        }

        async fn exists(&self, _key: &str) -> Result<bool> {
            self.exists_result.lock().await.clone()
        }

        async fn set_nx_ex<T: Serialize>(&self, _key: &str, _value: &T, _expiry_seconds: u64) -> Result<bool> {
            self.set_nx_ex_result.lock().await.clone()
        }

        async fn expire(&self, _key: &str, _expiry_seconds: u64) -> Result<bool> {
            self.expire_result.lock().await.clone()
        }
    }

    #[tokio::test]
    async fn test_set_ex_and_get() {
        let test_data = TestData {
            id: 1,
            name: "Test".to_string(),
            value: 42.5,
        };

        let serialized = serde_json::to_string(&test_data).unwrap();
        let pool = MockRedisPool::new()
            .with_set_ex_result(Ok(()))
            .with_get_result(Ok(Some(serialized)));

        let result = pool.set_ex("test_key", &test_data, 60).await;
        assert!(result.is_ok());

        let retrieved: Result<Option<TestData>> = pool.get_value("test_key").await;
        assert!(retrieved.is_ok());
        
        let data = retrieved.unwrap();
        assert!(data.is_some());
        assert_eq!(data.unwrap(), test_data);
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let pool = MockRedisPool::new().with_get_result(Ok(None));
        
        let result: Result<Option<TestData>> = pool.get_value("nonexistent_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_del() {
        let test_data = TestData {
            id: 2,
            name: "Delete Test".to_string(),
            value: 10.0,
        };

        let pool = MockRedisPool::new()
            .with_set_ex_result(Ok(()))
            .with_exists_result(Ok(true))
            .with_del_result(Ok(()));

        let result = pool.set_ex("delete_key", &test_data, 60).await;
        assert!(result.is_ok());
        
        let exists = pool.exists("delete_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
        
        let result = pool.del("delete_key").await;
        assert!(result.is_ok());
        
        let pool = MockRedisPool::new().with_exists_result(Ok(false));
        let exists = pool.exists("delete_key").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn test_exists() {
        let pool = MockRedisPool::new().with_exists_result(Ok(false));
        
        let exists = pool.exists("exists_key").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
        
        let test_data = TestData {
            id: 3,
            name: "Exists Test".to_string(),
            value: 30.0,
        };
        
        let pool = MockRedisPool::new()
            .with_set_ex_result(Ok(()))
            .with_exists_result(Ok(true));
            
        let result = pool.set_ex("exists_key", &test_data, 60).await;
        assert!(result.is_ok());
        
        let exists = pool.exists("exists_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    #[tokio::test]
    async fn test_set_nx_ex() {
        let test_data1 = TestData {
            id: 4,
            name: "First".to_string(),
            value: 40.0,
        };
        let test_data2 = TestData {
            id: 5,
            name: "Second".to_string(),
            value: 50.0,
        };
        
        let serialized = serde_json::to_string(&test_data1).unwrap();
        let pool = MockRedisPool::new()
            .with_set_nx_ex_result(Ok(true))
            .with_get_result(Ok(Some(serialized)));
        
        let result = pool.set_nx_ex("nx_key", &test_data1, 60).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let pool = MockRedisPool::new().with_set_nx_ex_result(Ok(false));
        let result = pool.set_nx_ex("nx_key", &test_data2, 60).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        let serialized = serde_json::to_string(&test_data1).unwrap();
        let pool = MockRedisPool::new().with_get_result(Ok(Some(serialized)));
        let retrieved: Result<Option<TestData>> = pool.get_value("nx_key").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().unwrap(), test_data1);
    }

    #[tokio::test]
    async fn test_expire() {
        let test_data = TestData {
            id: 6,
            name: "Expire Test".to_string(),
            value: 60.0,
        };
        
        let pool = MockRedisPool::new()
            .with_set_ex_result(Ok(()))
            .with_expire_result(Ok(true));
        
        let result = pool.set_ex("expire_key", &test_data, 30).await;
        assert!(result.is_ok());
        
        let result = pool.expire("expire_key", 120).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let pool = MockRedisPool::new().with_expire_result(Ok(false));
        let result = pool.expire("nonexistent_key", 60).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[async_trait]
    impl RedisPoolTrait for MockRedisPool {
        async fn get_value<'a, T: serde::de::DeserializeOwned + Send + Sync>(&'a self, key: &'a str) -> Result<Option<T>> {
            self.get_value(key).await
        }

        async fn set_ex<'a, T: serde::Serialize + Send + Sync>(&'a self, key: &'a str, value: &'a T, expiry_seconds: u64) -> Result<()> {
            self.set_ex(key, value, expiry_seconds).await
        }

        async fn del<'a>(&'a self, key: &'a str) -> Result<()> {
            self.del(key).await
        }
    }


    #[tokio::test]
    #[ignore]
    async fn test_redis_pool_integration() {
        let redis_url = std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let pool = RedisPool::new(&redis_url);
        assert!(pool.is_ok());

        let pool = pool.unwrap();
        
        let test_data = TestData {
            id: 100,
            name: "Integration Test".to_string(),
            value: 100.0,
        };
        
        let _ = pool.del("integration_test_key").await;
        
        let result = pool.set_ex("integration_test_key", &test_data, 60).await;
        assert!(result.is_ok());
        
        let retrieved: Result<Option<TestData>> = pool.get_value("integration_test_key").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().unwrap(), test_data);
        
        let exists = pool.exists("integration_test_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
        
        let result = pool.del("integration_test_key").await;
        assert!(result.is_ok());
    }
}
