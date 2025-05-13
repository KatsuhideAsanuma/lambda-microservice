
use crate::error::{Error, Result};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::time::Duration;

#[derive(Clone)]
pub struct RedisPool {
    pool: Pool,
}

impl RedisPool {
    pub fn new(redis_url: &str) -> Result<Self> {
        let mut config = Config::from_url(redis_url);
        config.pool = Some(deadpool_redis::PoolConfig::new(10));

        let pool = config.create_pool(Some(Runtime::Tokio1))?;

        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<deadpool_redis::Connection> {
        self.pool.get().await.map_err(Error::from)
    }

    pub async fn set_ex<T: serde::Serialize>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<()> {
        let mut conn = self.get().await?;
        let serialized = serde_json::to_string(value)?;
        conn.set_ex(key, serialized, expiry_seconds).await.map_err(Error::from)
    }

    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
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
            .arg(expiry_seconds)
            .query_async(&mut conn)
            .await
            .map_err(Error::from)?;
        Ok(result)
    }

    pub async fn expire(&self, key: &str, expiry_seconds: u64) -> Result<bool> {
        let mut conn = self.get().await?;
        let result: bool = conn.expire(key, expiry_seconds).await.map_err(Error::from)?;
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
    struct MockRedisConnection {
        storage: Arc<Mutex<std::collections::HashMap<String, (String, Option<u64>)>>>,
    }

    impl MockRedisConnection {
        fn new() -> Self {
            Self {
                storage: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }
        }

        async fn set_ex(&mut self, key: &str, value: String, expiry_seconds: u64) -> Result<()> {
            let mut storage = self.storage.lock().await;
            storage.insert(key.to_string(), (value, Some(expiry_seconds)));
            Ok(())
        }

        async fn get(&mut self, key: &str) -> Result<Option<String>> {
            let storage = self.storage.lock().await;
            Ok(storage.get(key).map(|(value, _)| value.clone()))
        }

        async fn del(&mut self, key: &str) -> Result<()> {
            let mut storage = self.storage.lock().await;
            storage.remove(key);
            Ok(())
        }

        async fn exists(&mut self, key: &str) -> Result<i64> {
            let storage = self.storage.lock().await;
            Ok(if storage.contains_key(key) { 1 } else { 0 })
        }

        async fn set_nx_ex(&mut self, key: &str, value: String, expiry_seconds: u64) -> Result<bool> {
            let mut storage = self.storage.lock().await;
            if storage.contains_key(key) {
                Ok(false)
            } else {
                storage.insert(key.to_string(), (value, Some(expiry_seconds)));
                Ok(true)
            }
        }

        async fn expire(&mut self, key: &str, expiry_seconds: u64) -> Result<bool> {
            let mut storage = self.storage.lock().await;
            if let Some((value, expiry)) = storage.get_mut(key) {
                *expiry = Some(expiry_seconds);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    struct MockRedisPool {
        connection: MockRedisConnection,
    }

    impl MockRedisPool {
        fn new() -> Self {
            Self {
                connection: MockRedisConnection::new(),
            }
        }

        async fn get(&self) -> Result<MockRedisConnection> {
            Ok(self.connection.clone())
        }

        async fn set_ex<T: Serialize>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<()> {
            let mut conn = self.get().await?;
            let serialized = serde_json::to_string(value)?;
            conn.set_ex(key, serialized, expiry_seconds).await
        }

        async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
            let mut conn = self.get().await?;
            let result = conn.get(key).await?;
            
            match result {
                Some(value) => Ok(Some(serde_json::from_str(&value)?)),
                None => Ok(None),
            }
        }

        async fn del(&self, key: &str) -> Result<()> {
            let mut conn = self.get().await?;
            conn.del(key).await
        }

        async fn exists(&self, key: &str) -> Result<bool> {
            let mut conn = self.get().await?;
            let result = conn.exists(key).await?;
            Ok(result > 0)
        }

        async fn set_nx_ex<T: Serialize>(&self, key: &str, value: &T, expiry_seconds: u64) -> Result<bool> {
            let mut conn = self.get().await?;
            let serialized = serde_json::to_string(value)?;
            conn.set_nx_ex(key, serialized, expiry_seconds).await
        }

        async fn expire(&self, key: &str, expiry_seconds: u64) -> Result<bool> {
            let mut conn = self.get().await?;
            conn.expire(key, expiry_seconds).await
        }
    }

    #[tokio::test]
    async fn test_set_ex_and_get() {
        let pool = MockRedisPool::new();
        let test_data = TestData {
            id: 1,
            name: "Test".to_string(),
            value: 42.5,
        };

        let result = pool.set_ex("test_key", &test_data, 60).await;
        assert!(result.is_ok());

        let retrieved: Result<Option<TestData>> = pool.get("test_key").await;
        assert!(retrieved.is_ok());
        
        let data = retrieved.unwrap();
        assert!(data.is_some());
        assert_eq!(data.unwrap(), test_data);
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let pool = MockRedisPool::new();
        
        let result: Result<Option<TestData>> = pool.get("nonexistent_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_del() {
        let pool = MockRedisPool::new();
        let test_data = TestData {
            id: 2,
            name: "Delete Test".to_string(),
            value: 10.0,
        };

        let _ = pool.set_ex("delete_key", &test_data, 60).await;
        
        let exists = pool.exists("delete_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
        
        let result = pool.del("delete_key").await;
        assert!(result.is_ok());
        
        let exists = pool.exists("delete_key").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[tokio::test]
    async fn test_exists() {
        let pool = MockRedisPool::new();
        
        let exists = pool.exists("exists_key").await;
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
        
        let test_data = TestData {
            id: 3,
            name: "Exists Test".to_string(),
            value: 30.0,
        };
        let _ = pool.set_ex("exists_key", &test_data, 60).await;
        
        let exists = pool.exists("exists_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    #[tokio::test]
    async fn test_set_nx_ex() {
        let pool = MockRedisPool::new();
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
        
        let result = pool.set_nx_ex("nx_key", &test_data1, 60).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let result = pool.set_nx_ex("nx_key", &test_data2, 60).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        let retrieved: Result<Option<TestData>> = pool.get("nx_key").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().unwrap(), test_data1);
    }

    #[tokio::test]
    async fn test_expire() {
        let pool = MockRedisPool::new();
        let test_data = TestData {
            id: 6,
            name: "Expire Test".to_string(),
            value: 60.0,
        };
        
        let _ = pool.set_ex("expire_key", &test_data, 30).await;
        
        let result = pool.expire("expire_key", 120).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let result = pool.expire("nonexistent_key", 60).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
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
        
        let retrieved: Result<Option<TestData>> = pool.get("integration_test_key").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().unwrap(), test_data);
        
        let exists = pool.exists("integration_test_key").await;
        assert!(exists.is_ok());
        assert!(exists.unwrap());
        
        let result = pool.del("integration_test_key").await;
        assert!(result.is_ok());
    }
}
