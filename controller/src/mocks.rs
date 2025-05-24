
use crate::error::{Error, Result};
use crate::session::DbPoolTrait;
use crate::logger::DatabaseLoggerTrait;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;
use serde::{Serialize, Deserialize};
use std::pin::Pin;
use std::future::Future;

#[derive(Clone)]
pub struct MockDatabaseLogger {
    log_error_result: Arc<Mutex<Result<()>>>,
    log_request_result: Arc<Mutex<Result<()>>>,
}

impl MockDatabaseLogger {
    pub fn new() -> Self {
        Self {
            log_error_result: Arc::new(Mutex::new(Ok(()))),
            log_request_result: Arc::new(Mutex::new(Ok(()))),
        }
    }

    pub fn with_log_error_result(mut self, result: Result<()>) -> Self {
        self.log_error_result = Arc::new(Mutex::new(result));
        self
    }
    
    pub fn with_log_request_result(mut self, result: Result<()>) -> Self {
        self.log_request_result = Arc::new(Mutex::new(result));
        self
    }
}

#[async_trait]
impl DatabaseLoggerTrait for MockDatabaseLogger {
    fn log_request(
        &self,
        _request_id: String,
        _language_title: String,
        _client_ip: Option<String>,
        _user_id: Option<String>,
        _request_headers: Option<serde_json::Value>,
        _request_payload: Option<serde_json::Value>,
        _response_payload: Option<serde_json::Value>,
        _status_code: i32,
        _duration_ms: i64,
        _cached: bool,
        _error_details: Option<serde_json::Value>,
        _runtime_metrics: Option<serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let result = self.log_request_result.clone();
        Box::pin(async move {
            result.lock().await.clone()
        })
    }
    
    fn log_error(
        &self,
        _request_log_id: String,
        _error_code: String,
        _error_message: String,
        _stack_trace: Option<String>,
        _context: Option<serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let result = self.log_error_result.clone();
        Box::pin(async move {
            result.lock().await.clone()
        })
    }
}

#[derive(Clone)]
pub struct MockPostgresPool {
    execute_result: Arc<Mutex<Result<u64>>>,
    query_opt_result: Arc<Mutex<Result<Option<Row>>>>,
    query_one_result: Arc<Mutex<Result<Row>>>,
}

impl MockPostgresPool {
    pub fn new() -> Self {
        Self {
            execute_result: Arc::new(Mutex::new(Ok(1))),
            query_opt_result: Arc::new(Mutex::new(Ok(None))),
            query_one_result: Arc::new(Mutex::new(Err(Error::NotFound("No rows found".to_string())))),
        }
    }

    pub fn with_execute_result(mut self, result: Result<u64>) -> Self {
        self.execute_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
        self.query_opt_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_query_one_result(mut self, result: Result<Row>) -> Self {
        self.query_one_result = Arc::new(Mutex::new(result));
        self
    }

    pub async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.execute_result.lock().await.clone()
    }

    pub async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
        Ok(Vec::new())
    }

    pub async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
        self.query_one_result.lock().await.clone()
    }

    pub async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        self.query_opt_result.lock().await.clone()
    }
}

#[async_trait]
impl DbPoolTrait for MockPostgresPool {
    async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.execute(query, params).await
    }
    
    async fn query_opt<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        self.query_opt(query, params).await
    }
    
    async fn query_one<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
        self.query_one(query, params).await
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

    pub async fn get_value<T: serde::de::DeserializeOwned + Send + Sync>(&self, _key: &str) -> Result<Option<T>> {
        let result = self.get_result.lock().await.clone()?;
        match result {
            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
            None => Ok(None),
        }
    }

    pub async fn set_ex<T: Serialize + Send + Sync>(&self, _key: &str, _value: &T, _expiry_seconds: u64) -> Result<()> {
        self.set_ex_result.lock().await.clone()
    }

    pub async fn del(&self, _key: &str) -> Result<()> {
        self.del_result.lock().await.clone()
    }

    pub async fn exists(&self, _key: &str) -> Result<bool> {
        self.exists_result.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl crate::cache::RedisPoolTrait for MockRedisPool {
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
