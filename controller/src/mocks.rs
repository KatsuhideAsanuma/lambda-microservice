
use crate::error::{Error, Result};
use crate::session::DbPoolTrait;
use crate::logger::DatabaseLoggerTrait;
use crate::runtime::{RuntimeExecuteResponse, RuntimeType};
use crate::session::Session;
use crate::openfaas::{OpenFaaSRequest, OpenFaaSResponse};
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

#[derive(Clone)]
pub struct MockOpenFaaSClient {
    invoke_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
}

impl MockOpenFaaSClient {
    pub fn new() -> Self {
        Self {
            invoke_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                result: serde_json::json!({"status": "success"}),
                execution_time_ms: 100,
                memory_usage_bytes: Some(1024),
            }))),
        }
    }
    
    pub fn with_invoke_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
        self.invoke_result = Arc::new(Mutex::new(result));
        self
    }
    
    pub async fn invoke_function(
        &self,
        _function_name: &str,
        _session: &Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.invoke_result.lock().await.clone()
    }
    
    pub fn get_function_name_for_runtime(&self, runtime_type: RuntimeType) -> String {
        match runtime_type {
            RuntimeType::NodeJs => "nodejs-runtime".to_string(),
            RuntimeType::Python => "python-runtime".to_string(),
            RuntimeType::Rust => "rust-runtime".to_string(),
        }
    }
    
    pub fn build_request(&self, _function_name: &str, session: &Session, params: serde_json::Value) -> OpenFaaSRequest {
        OpenFaaSRequest {
            request_id: session.request_id.clone(),
            params,
            context: session.context.clone(),
            script_content: session.script_content.clone(),
        }
    }
}

use crate::api::RuntimeManagerTrait;
use crate::runtime::RuntimeConfig;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct MockRuntimeManager {
    execute_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
    compile_result: Arc<Mutex<Result<Vec<u8>>>>,
    config: RuntimeConfig,
    call_count: Arc<AtomicUsize>,
}

impl MockRuntimeManager {
    pub fn new() -> Self {
        Self {
            execute_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                result: serde_json::json!({"status": "success"}),
                execution_time_ms: 100,
                memory_usage_bytes: Some(1024),
            }))),
            compile_result: Arc::new(Mutex::new(Ok(vec![0, 1, 2, 3]))), // Mock WASM binary
            config: RuntimeConfig {
                nodejs_url: "http://localhost:8081".to_string(),
                python_url: "http://localhost:8082".to_string(),
                rust_url: "http://localhost:8083".to_string(),
                timeout_seconds: 30,
                fallback_timeout_seconds: 15,
                max_retries: 3,
                wasm_compile_timeout_seconds: 60,
                wasm_temp_dir: "/tmp".to_string(),
            },
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn with_execute_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
        self.execute_result = Arc::new(Mutex::new(result));
        self
    }
    
    pub fn with_compile_result(mut self, result: Result<Vec<u8>>) -> Self {
        self.compile_result = Arc::new(Mutex::new(result));
        self
    }
    
    pub fn get_call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl RuntimeManagerTrait for MockRuntimeManager {
    async fn execute<'a>(
        &'a self,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.execute_result.lock().await.clone()
    }
    
    async fn compile_rust_script<'a>(&'a self, _session: &'a Session) -> Result<Vec<u8>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.compile_result.lock().await.clone()
    }
    
    async fn execute_wasm<'a>(
        &'a self,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.execute_result.lock().await.clone()
    }
    
    async fn execute_in_container<'a>(
        &'a self,
        _runtime_type: RuntimeType,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.execute_result.lock().await.clone()
    }
    
    async fn compile_with_wasmtime<'a>(
        &'a self,
        _script_content: &'a str,
        _memory_limit_bytes: u64,
    ) -> Result<Vec<u8>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.compile_result.lock().await.clone()
    }
    
    #[cfg(test)]
    fn get_config(&self) -> &RuntimeConfig {
        &self.config
    }
}
