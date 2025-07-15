use super::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TestData {
    pub id: u64,
    pub name: String,
    pub value: f64,
}

#[derive(Clone)]
pub struct MockRedisConnection {
    get_result: Arc<Mutex<Result<Option<String>>>>,
    set_ex_result: Arc<Mutex<Result<()>>>,
    del_result: Arc<Mutex<Result<u64>>>,
    exists_result: Arc<Mutex<Result<bool>>>,
    set_nx_ex_result: Arc<Mutex<Result<bool>>>,
    expire_result: Arc<Mutex<Result<bool>>>,
}

impl MockRedisConnection {
    pub fn new() -> Self {
        Self {
            get_result: Arc::new(Mutex::new(Ok(None))),
            set_ex_result: Arc::new(Mutex::new(Ok(()))),
            del_result: Arc::new(Mutex::new(Ok(0))),
            exists_result: Arc::new(Mutex::new(Ok(false))),
            set_nx_ex_result: Arc::new(Mutex::new(Ok(true))),
            expire_result: Arc::new(Mutex::new(Ok(true))),
        }
    }

    pub async fn set_ex(&self, _key: &str, _value: &str, _seconds: usize) -> Result<()> {
        self.set_ex_result.lock().await.clone()
    }

    pub async fn get(&self, _key: &str) -> Result<Option<String>> {
        self.get_result.lock().await.clone()
    }

    pub async fn del(&self, _key: &str) -> Result<u64> {
        self.del_result.lock().await.clone()
    }

    pub async fn exists(&self, _key: &str) -> Result<bool> {
        self.exists_result.lock().await.clone()
    }

    pub async fn set_nx_ex(&self, _key: &str, _value: &str, _seconds: usize) -> Result<bool> {
        self.set_nx_ex_result.lock().await.clone()
    }

    pub async fn expire(&self, _key: &str, _seconds: usize) -> Result<bool> {
        self.expire_result.lock().await.clone()
    }
}

#[derive(Clone)]
pub struct MockRedisPool {
    get_result: Arc<Mutex<Result<Option<String>>>>,
    set_ex_result: Arc<Mutex<Result<()>>>,
    del_result: Arc<Mutex<Result<u64>>>,
    exists_result: Arc<Mutex<Result<bool>>>,
    set_nx_ex_result: Arc<Mutex<Result<bool>>>,
    expire_result: Arc<Mutex<Result<bool>>>,
}

impl MockRedisPool {
    pub fn new() -> Self {
        Self {
            get_result: Arc::new(Mutex::new(Ok(None))),
            set_ex_result: Arc::new(Mutex::new(Ok(()))),
            del_result: Arc::new(Mutex::new(Ok(0))),
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

    pub fn with_del_result(mut self, result: Result<u64>) -> Self {
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

    pub async fn get_value<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let result = self.get_result.lock().await.clone()?;
        match result {
            Some(value) => {
                let deserialized: T = serde_json::from_str(&value)?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    pub async fn set_ex<T: Serialize>(&self, _key: &str, _value: &T, _seconds: usize) -> Result<()> {
        self.set_ex_result.lock().await.clone()
    }

    pub async fn del(&self, _key: &str) -> Result<u64> {
        self.del_result.lock().await.clone()
    }

    pub async fn exists(&self, _key: &str) -> Result<bool> {
        self.exists_result.lock().await.clone()
    }

    pub async fn set_nx_ex<T: Serialize>(&self, _key: &str, _value: &T, _seconds: usize) -> Result<bool> {
        self.set_nx_ex_result.lock().await.clone()
    }

    pub async fn expire(&self, _key: &str, _seconds: usize) -> Result<bool> {
        self.expire_result.lock().await.clone()
    }
}

impl RedisPoolTrait for MockRedisPool {
    async fn get_value<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        self.get_value(key).await
    }

    async fn set_ex<T: Serialize>(&self, key: &str, value: &T, seconds: usize) -> Result<()> {
        self.set_ex(key, value, seconds).await
    }

    async fn del(&self, key: &str) -> Result<u64> {
        self.del(key).await
    }
}

pub fn create_test_redis_client() -> RedisClient {
    RedisClient {
        pool: MockRedisPool::new(),
        ttl_seconds: 3600,
    }
}
