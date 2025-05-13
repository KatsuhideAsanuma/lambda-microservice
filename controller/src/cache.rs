
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
