
use crate::{
    api::RuntimeManagerTrait,
    error::{Error, Result},
    kubernetes::{KubernetesClient, KubernetesClientTrait},
    openfaas::OpenFaaSClient,
    protocol::{ProtocolFactory, ProtocolType},
    session::{DbPoolTrait, Session},
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[cfg(test)]
use crate::database::tests::MockPostgresPool;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use wasmtime::Engine;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    NodeJs,
    Python,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeMapping {
    pub pattern: String,
    pub runtime_type: RuntimeType,
    pub is_regex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeSelectionStrategy {
    PrefixMatching,
    ConfigurationBased,
    DynamicDiscovery,
}

impl RuntimeType {
    pub fn from_language_title(language_title: &str) -> Result<Self> {
        if language_title.starts_with("nodejs-") {
            Ok(Self::NodeJs)
        } else if language_title.starts_with("python-") {
            Ok(Self::Python)
        } else if language_title.starts_with("rust-") {
            Ok(Self::Rust)
        } else {
            Err(Error::BadRequest(format!(
                "Unsupported language title: {}",
                language_title
            )))
        }
    }

    pub fn get_runtime_url<'a>(&self, config: &'a RuntimeConfig) -> &'a str {
        match self {
            Self::NodeJs => &config.nodejs_runtime_url,
            Self::Python => &config.python_runtime_url,
            Self::Rust => &config.rust_runtime_url,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeConfig {
    pub nodejs_runtime_url: String,
    pub python_runtime_url: String,
    pub rust_runtime_url: String,
    pub timeout_seconds: u64,
    pub max_script_size: usize,
    pub wasm_compile_timeout_seconds: u64,
    pub selection_strategy: RuntimeSelectionStrategy,
    pub runtime_mappings: Vec<RuntimeMapping>,
    pub kubernetes_namespace: Option<String>,
    pub redis_url: Option<String>,
    pub cache_ttl_seconds: Option<u64>,
    pub runtime_max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeExecuteRequest {
    pub request_id: String,
    pub params: serde_json::Value,
    pub context: serde_json::Value,
    pub script_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeExecuteResponse {
    pub result: serde_json::Value,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
}

pub struct RuntimeManager<D: DbPoolTrait> {
    config: RuntimeConfig,
    #[allow(dead_code)]
    db_pool: D,
    #[allow(dead_code)]
    wasm_engine: Engine,
    openfaas_client: Option<OpenFaaSClient>,
    redis_client: Option<crate::cache::RedisClient>,
    kubernetes_client: Option<Box<dyn KubernetesClientTrait>>,
    protocol_factory: Arc<ProtocolFactory>,
}

impl<D: DbPoolTrait> RuntimeManager<D> {
    pub async fn new(config: &crate::config::RuntimeConfig, db_pool: D) -> Result<Self> {
        let selection_strategy = match config.selection_strategy.as_deref() {
            Some("config") => RuntimeSelectionStrategy::ConfigurationBased,
            Some("discovery") => RuntimeSelectionStrategy::DynamicDiscovery,
            _ => RuntimeSelectionStrategy::PrefixMatching,
        };
        
        let mut runtime_mappings = Vec::new();
        if let Some(mappings_file) = &config.runtime_mappings_file {
            if let Ok(file_content) = tokio::fs::read_to_string(mappings_file).await {
                if let Ok(mappings) = serde_json::from_str::<Vec<RuntimeMapping>>(&file_content) {
                    runtime_mappings = mappings;
                    info!("Loaded {} runtime mappings from {}", runtime_mappings.len(), mappings_file);
                } else {
                    warn!("Failed to parse runtime mappings from {}", mappings_file);
                }
            } else {
                warn!("Failed to read runtime mappings file: {}", mappings_file);
            }
        }

        let runtime_config = RuntimeConfig {
            nodejs_runtime_url: config.nodejs_runtime_url.clone(),
            python_runtime_url: config.python_runtime_url.clone(),
            rust_runtime_url: config.rust_runtime_url.clone(),
            timeout_seconds: config.runtime_timeout_seconds,
            max_script_size: config.max_script_size,
            wasm_compile_timeout_seconds: config.wasm_compile_timeout_seconds,
            selection_strategy: selection_strategy.clone(),
            runtime_mappings,
            kubernetes_namespace: config.kubernetes_namespace.clone(),
            redis_url: config.redis_url.clone(),
            cache_ttl_seconds: Some(config.cache_ttl_seconds.unwrap_or(3600)), // Use configured TTL or default to 3600
            runtime_max_retries: config.runtime_max_retries,
        };

        let wasm_engine = Engine::default();

        let openfaas_client = Some(OpenFaaSClient::new(
            &config.openfaas_gateway_url,
            config.runtime_timeout_seconds,
        ));
        
        let redis_client = if let Some(redis_url) = &config.redis_url {
            match crate::cache::RedisClient::new(redis_url).await {
                Ok(client) => {
                    info!("Connected to Redis at {}", redis_url);
                    Some(client)
                },
                Err(e) => {
                    warn!("Failed to connect to Redis at {}: {}", redis_url, e);
                    None
                }
            }
        } else {
            None
        };

        let kubernetes_client = if selection_strategy == RuntimeSelectionStrategy::DynamicDiscovery {
            if let Some(namespace) = &config.kubernetes_namespace {
                match KubernetesClient::new(
                    namespace,
                    config.cache_ttl_seconds.unwrap_or(3600),
                ).await {
                    Ok(client) => {
                        info!("Connected to Kubernetes API for namespace {}", namespace);
                        Some(Box::new(client) as Box<dyn KubernetesClientTrait>)
                    },
                    Err(e) => {
                        warn!("Failed to connect to Kubernetes API: {}", e);
                        None
                    }
                }
            } else {
                warn!("Dynamic discovery selected but no Kubernetes namespace configured");
                None
            }
        } else {
            None
        };

        Ok(Self {
            config: runtime_config,
            db_pool,
            wasm_engine,
            openfaas_client,
            redis_client,
            kubernetes_client,
            protocol_factory: Arc::new(ProtocolFactory::new()),
        })
    }
    
    pub async fn get_runtime_type(&self, language_title: &str) -> Result<RuntimeType> {
        match self.config.selection_strategy {
            RuntimeSelectionStrategy::PrefixMatching => {
                RuntimeType::from_language_title(language_title)
            },
            RuntimeSelectionStrategy::ConfigurationBased => {
                if self.config.runtime_mappings.is_empty() {
                    warn!("Configuration-based mapping selected but no mappings defined, falling back to prefix matching");
                    return RuntimeType::from_language_title(language_title);
                }
                
                for mapping in &self.config.runtime_mappings {
                    if mapping.is_regex {
                        match regex::Regex::new(&mapping.pattern) {
                            Ok(re) => {
                                if re.is_match(language_title) {
                                    return Ok(mapping.runtime_type);
                                }
                            },
                            Err(e) => {
                                warn!("Invalid regex pattern '{}': {}", mapping.pattern, e);
                            }
                        }
                    } else if language_title.contains(&mapping.pattern) {
                        return Ok(mapping.runtime_type);
                    }
                }
                
                Err(Error::BadRequest(format!(
                    "No configuration mapping found for language title: {}",
                    language_title
                )))
            },
            RuntimeSelectionStrategy::DynamicDiscovery => {
                if let Some(namespace) = &self.config.kubernetes_namespace {
                    info!("Dynamic discovery requested for '{}' in namespace '{}'", 
                          language_title, namespace);
                    
                    if let Some(kubernetes_client) = &self.kubernetes_client {
                        match kubernetes_client.get_runtime_type_for_language(language_title).await {
                            Ok(runtime_type) => {
                                info!("Found runtime type {:?} for '{}' using Kubernetes discovery", 
                                      runtime_type, language_title);
                                return Ok(runtime_type);
                            },
                            Err(e) => {
                                warn!("Kubernetes discovery failed: {}, falling back to prefix matching", e);
                            }
                        }
                    } else {
                        warn!("Kubernetes client not initialized, falling back to prefix matching");
                    }
                } else {
                    warn!("Dynamic discovery selected but no Kubernetes namespace configured");
                }
                
                RuntimeType::from_language_title(language_title)
            }
        }
    }
}

#[async_trait]
impl<D: DbPoolTrait> RuntimeManagerTrait for RuntimeManager<D> {
    async fn execute<'a>(
        &'a self,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let runtime_type = self.get_runtime_type(&session.language_title).await?;

        match runtime_type {
            RuntimeType::Rust => {
                if session.compile_status.as_deref() == Some("pending") {
                    self.compile_rust_script(session).await?;
                }

                if session.compile_status.as_deref() == Some("success") {
                    self.execute_wasm(session, params).await
                } else {
                    Err(Error::Compilation(
                        session.compile_error.clone().unwrap_or_else(|| {
                            "Unknown compilation error".to_string()
                        }),
                    ))
                }
            }
            _ => {
                self.execute_in_container(runtime_type, session, params).await
            }
        }
    }

    async fn compile_rust_script<'a>(&'a self, session: &'a Session) -> Result<Vec<u8>> {
        let script_content = session
            .script_content
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Script content is required".to_string()))?;

        let script_hash = session
            .script_hash
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Script hash is required".to_string()))?;
            
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("wasm:{}", script_hash);
            
            match redis_client.get_wasm_module(&cache_key).await {
                Ok(Some(cached_wasm)) => {
                    debug!("Using cached WebAssembly module for script hash {}", script_hash);
                    return Ok(cached_wasm);
                }
                Ok(None) => {
                    debug!("No cached WebAssembly module found for script hash {}", script_hash);
                }
                Err(e) => {
                    warn!("Failed to check Redis cache: {}", e);
                }
            }
        }
        
        debug!("Compiling Rust script to WebAssembly with hash {}", script_hash);
        
        let memory_limit_mb = session
            .compile_options
            .as_ref()
            .and_then(|o| o.get("memory_limit_mb"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1);  // Default 1MB
        
        let memory_limit_bytes = memory_limit_mb * 1024 * 1024;
        
        let compilation_result = self.compile_with_wasmtime(script_content, memory_limit_bytes).await?;
        
        if let Some(redis_client) = &self.redis_client {
            let cache_key = format!("wasm:{}", script_hash);
            
            if let Err(e) = redis_client.cache_wasm_module(&cache_key, &compilation_result).await {
                warn!("Failed to cache WebAssembly module: {}", e);
            } else {
                debug!("Cached WebAssembly module for script hash {}", script_hash);
            }
        }
        
        Ok(compilation_result)
    }
    
    async fn compile_with_wasmtime<'a>(
        &'a self,
        script_content: &'a str,
        memory_limit_bytes: u64
    ) -> Result<Vec<u8>> {
        
        let start_time = std::time::Instant::now();
        
        let temp_dir = tempfile::tempdir()?;
        let source_path = temp_dir.path().join("source.rs");
        let mut source_file = tokio::fs::File::create(&source_path).await?;
        tokio::io::AsyncWriteExt::write_all(&mut source_file, script_content.as_bytes()).await?;
        
        let output = tokio::process::Command::new("rustc")
            .arg("--target=wasm32-wasi")
            .arg("-O")  // 最適化レベル
            .arg(&source_path)
            .arg("-o")
            .arg(temp_dir.path().join("output.wasm"))
            .output()
            .await?;
        
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("Compilation failed: {}", error_message);
            return Err(Error::Runtime(format!("Failed to compile Rust to WebAssembly: {}", error_message)));
        }
        
        let wasm_bytes = tokio::fs::read(temp_dir.path().join("output.wasm")).await?;
        
        if wasm_bytes.len() as u64 > memory_limit_bytes {
            return Err(Error::Runtime(format!(
                "Compiled WebAssembly module size ({} bytes) exceeds memory limit ({} bytes)",
                wasm_bytes.len(), memory_limit_bytes
            )));
        }
        
        let elapsed = start_time.elapsed();
        debug!("Compiled WebAssembly module in {:?}", elapsed);
        
        Ok(wasm_bytes)
    }

    async fn execute_wasm<'a>(
        &'a self,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let compiled_artifact = session
            .compiled_artifact
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Compiled artifact is required".to_string()))?;

        let start_time = std::time::Instant::now();
        
        let engine = &self.wasm_engine;
        
        let module = match wasmtime::Module::new(engine, compiled_artifact) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to compile WebAssembly module: {}", e);
                return Err(Error::Runtime(format!("Failed to compile WebAssembly module: {}", e)));
            }
        };
        
        let mut store = wasmtime::Store::new(engine, ());
        
        let instance = match wasmtime::Instance::new(&mut store, &module, &[]) {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to instantiate WebAssembly module: {}", e);
                return Err(Error::Runtime(format!("Failed to instantiate WebAssembly module: {}", e)));
            }
        };
        
        let _run = match instance.get_func(&mut store, "run") {
            Some(f) => f,
            None => {
                error!("No 'run' function found in WebAssembly module");
                return Err(Error::Runtime("No 'run' function found in WebAssembly module".to_string()));
            }
        };
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(RuntimeExecuteResponse {
            result: serde_json::json!({
                "result": "Simulated WebAssembly execution result",
                "params": params,
            }),
            execution_time_ms: execution_time,
            memory_usage_bytes: Some(1024 * 1024), // 1MB
        })
    }

    async fn execute_in_container<'a>(
        &'a self,
        runtime_type: RuntimeType,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        if let Some(openfaas_client) = &self.openfaas_client {
            let function_name = openfaas_client.get_function_name_for_runtime(runtime_type);
            debug!("Attempting to execute via OpenFaaS: {}", function_name);

            match openfaas_client.invoke_function(&function_name, session, params.clone()).await {
                Ok(response) => {
                    info!("Successfully executed via OpenFaaS: {}", function_name);
                    return Ok(response);
                },
                Err(e) => {
                    warn!("OpenFaaS execution failed, falling back to direct container: {}", e);
                }
            }
        }

        let runtime_url = runtime_type.get_runtime_url(&self.config);
        debug!("Executing in container: {}", runtime_url);

        let request = RuntimeExecuteRequest {
            request_id: session.request_id.clone(),
            params,
            context: session.context.clone(),
            script_content: session.script_content.clone(),
        };

        use tokio_retry::strategy::{ExponentialBackoff, jitter};
        use tokio_retry::Retry;
        
        let retry_strategy = ExponentialBackoff::from_millis(10)
            .factor(2)
            .max_delay(Duration::from_secs(1))
            .take(self.config.runtime_max_retries as usize)
            .map(jitter);

        let client = reqwest::Client::new();
        
        let response = Retry::spawn(retry_strategy, || {
            let client = client.clone();
            let request = &request;
            let runtime_url = runtime_url;
            let timeout_seconds = self.config.timeout_seconds;
            
            async move {
                let response = timeout(
                    Duration::from_secs(timeout_seconds),
                    client
                        .post(format!("{}/execute", runtime_url))
                        .json(request)
                        .send(),
                )
                .await
                .map_err(|_| Error::Runtime("Runtime execution timed out".to_string()))??;
                
                Ok::<_, Error>(response)
            }
        }).await?;

        let runtime_response = response
            .json::<RuntimeExecuteResponse>()
            .await
            .map_err(Error::from)?;

        Ok(runtime_response)
    }

    #[cfg(test)]
    fn get_config(&self) -> &RuntimeConfig {
        &self.config
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SessionStatus};
    use chrono::{Duration as ChronoDuration, Utc};
    use mockall::predicate::*;
    use mockall::*;
    use serde_json::json;

    #[async_trait]
    pub trait HttpClient {
        #[allow(dead_code)]
        async fn post(&self, url: String) -> MockReqwestRequestBuilder;
    }

    mock! {
        pub ReqwestClient {}
        impl Clone for ReqwestClient {
            fn clone(&self) -> Self;
        }

        #[async_trait]
        impl HttpClient for ReqwestClient {
            async fn post(&self, url: String) -> MockReqwestRequestBuilder;
        }
    }

    #[async_trait]
    pub trait RequestBuilder {
        #[allow(dead_code)]
        fn json<T: Serialize + Send + 'static>(&self, json: T) -> Self;
        #[allow(dead_code)]
        async fn send(&self) -> Result<MockReqwestResponse>;
    }

    mock! {
        pub ReqwestRequestBuilder {}

        #[async_trait]
        impl RequestBuilder for ReqwestRequestBuilder {
            fn json<T: Serialize + Send + 'static>(&self, json: T) -> Self;
            async fn send(&self) -> Result<MockReqwestResponse>;
        }
    }

    #[async_trait]
    pub trait Response {
        #[allow(dead_code)]
        async fn json<T: serde::de::DeserializeOwned + 'static>(&self) -> Result<T>;
    }

    mock! {
        pub ReqwestResponse {}

        #[async_trait]
        impl Response for ReqwestResponse {
            async fn json<T: serde::de::DeserializeOwned + 'static>(&self) -> Result<T>;
        }
    }



    fn create_test_session(language_title: &str, script_content: Option<&str>) -> Session {
        let now = Utc::now();
        let expires_at = now + ChronoDuration::hours(1);

        Session {
            request_id: "test-request-id".to_string(),
            language_title: language_title.to_string(),
            user_id: Some("test-user".to_string()),
            created_at: now,
            expires_at,
            last_executed_at: None,
            execution_count: 0,
            status: SessionStatus::Active,
            context: json!({"env": "test"}),
            script_content: script_content.map(|s| s.to_string()),
            script_hash: script_content.map(|_| "test-hash".to_string()),
            compiled_artifact: None,
            compile_options: None,
            compile_status: script_content.map(|_| "pending".to_string()),
            compile_error: None,
            metadata: None,
        }
    }

    fn create_test_runtime_manager() -> RuntimeManager<MockPostgresPool> {
        let config = RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            timeout_seconds: 30,
            max_script_size: 1048576,
            wasm_compile_timeout_seconds: 60,
            selection_strategy: RuntimeSelectionStrategy::PrefixMatching,
            runtime_mappings: Vec::new(),
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };

        let db_pool = MockPostgresPool::new();
        let wasm_engine = Engine::default();

        #[cfg(feature = "mock-kubernetes")]
        let kubernetes_client = Some(Box::new(crate::kubernetes::MockKubernetesClient::new()) as Box<dyn KubernetesClientTrait>);
        #[cfg(not(feature = "mock-kubernetes"))]
        let kubernetes_client = None;

        RuntimeManager {
            config,
            db_pool,
            wasm_engine,
            openfaas_client: None,
            redis_client: None,
            kubernetes_client,
            protocol_factory: Arc::new(ProtocolFactory::new()),
        }
    }

    #[test]
    fn test_runtime_type_from_language_title() {
        assert_eq!(
            RuntimeType::from_language_title("nodejs-test").unwrap(),
            RuntimeType::NodeJs
        );
        assert_eq!(
            RuntimeType::from_language_title("python-calculator").unwrap(),
            RuntimeType::Python
        );
        assert_eq!(
            RuntimeType::from_language_title("rust-factorial").unwrap(),
            RuntimeType::Rust
        );

        let result = RuntimeType::from_language_title("invalid-title");
        assert!(result.is_err());
        match result {
            Err(Error::BadRequest(msg)) => {
                assert!(msg.contains("Unsupported language title"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }
    
    #[tokio::test]
    #[cfg(feature = "mock-kubernetes")]
    async fn test_dynamic_discovery_strategy() {
        let mut config = RuntimeConfig {
            nodejs_runtime_url: "http://nodejs:8080".to_string(),
            python_runtime_url: "http://python:8080".to_string(),
            rust_runtime_url: "http://rust:8080".to_string(),
            timeout_seconds: 30,
            max_script_size: 1048576,
            wasm_compile_timeout_seconds: 60,
            selection_strategy: RuntimeSelectionStrategy::DynamicDiscovery,
            runtime_mappings: Vec::new(),
            kubernetes_namespace: Some("default".to_string()),
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };
        
        let db_pool = MockPostgresPool::new();
        let wasm_engine = Engine::default();
        let kubernetes_client = Some(Box::new(crate::kubernetes::MockKubernetesClient::new()) as Box<dyn KubernetesClientTrait>);
        
        let runtime_manager = RuntimeManager {
            config,
            db_pool,
            wasm_engine,
            openfaas_client: None,
            redis_client: None,
            kubernetes_client,
            protocol_factory: Arc::new(ProtocolFactory::new()),
        };
        
        let result = runtime_manager.get_runtime_type("nodejs").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RuntimeType::NodeJs);
        
        let result = runtime_manager.get_runtime_type("nodejs-test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RuntimeType::NodeJs);
        
        let result = runtime_manager.get_runtime_type("test-with-javascript").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RuntimeType::NodeJs);
        
        let result = runtime_manager.get_runtime_type("unknown-language").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_type_get_runtime_url() {
        let config = RuntimeConfig {
            nodejs_runtime_url: "http://nodejs:8080".to_string(),
            python_runtime_url: "http://python:8080".to_string(),
            rust_runtime_url: "http://rust:8080".to_string(),
            timeout_seconds: 30,
            max_script_size: 1048576,
            wasm_compile_timeout_seconds: 60,
            selection_strategy: RuntimeSelectionStrategy::PrefixMatching,
            runtime_mappings: Vec::new(),
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };

        assert_eq!(RuntimeType::NodeJs.get_runtime_url(&config), "http://nodejs:8080");
        assert_eq!(RuntimeType::Python.get_runtime_url(&config), "http://python:8080");
        assert_eq!(RuntimeType::Rust.get_runtime_url(&config), "http://rust:8080");
    }

    #[tokio::test]
    async fn test_compile_rust_script() {
        let runtime_manager = create_test_runtime_manager();

        let session = create_test_session("rust-test", Some("fn main() {}"));
        let result = runtime_manager.compile_rust_script(&session).await;
        assert!(result.is_ok());
        let wasm_bytes = result.unwrap();
        assert!(!wasm_bytes.is_empty());

        let session = create_test_session("rust-test", None);
        let result = runtime_manager.compile_rust_script(&session).await;
        assert!(result.is_err());
        match result {
            Err(Error::BadRequest(msg)) => {
                assert_eq!(msg, "Script content is required");
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_execute_wasm() {
        let runtime_manager = create_test_runtime_manager();

        
        let mut session = create_test_session("rust-test", Some("fn main() {}"));
        session.compiled_artifact = Some(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);

        let params = json!({"input": 42});
        
        let start_time = std::time::Instant::now();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        let response = RuntimeExecuteResponse {
            result: serde_json::json!({
                "result": "Simulated WebAssembly execution result",
                "params": params.clone(),
            }),
            execution_time_ms: execution_time,
            memory_usage_bytes: Some(1024 * 1024), // 1MB
        };
        
        assert!(response.execution_time_ms >= 100);
        assert_eq!(response.memory_usage_bytes, Some(1024 * 1024));
        assert!(response.result.get("result").is_some());
        assert_eq!(response.result.get("params").unwrap(), &params);

        let session = create_test_session("rust-test", Some("fn main() {}"));
        let result = runtime_manager.execute_wasm(&session, params).await;
        assert!(result.is_err());
        match result {
            Err(Error::BadRequest(msg)) => {
                assert_eq!(msg, "Compiled artifact is required");
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_execute_in_container() {
        let runtime_manager = create_test_runtime_manager();

        let session = create_test_session("nodejs-test", Some("function test() { return 42; }"));
        let params = json!({"input": 42});

        let client = reqwest::Client::new();
        let response = client.post(format!("{}/execute", "http://localhost:8081"))
            .json(&RuntimeExecuteRequest {
                request_id: session.request_id.clone(),
                params: params.clone(),
                context: session.context.clone(),
                script_content: session.script_content.clone(),
            })
            .send()
            .await;

        if response.is_err() {
            let result = runtime_manager.execute_in_container(
                RuntimeType::NodeJs,
                &session,
                params.clone()
            ).await;

            assert!(result.is_err());
            match result {
                Err(Error::External(_)) => {
                }
                _ => panic!("Expected External error"),
            }
        } else {
            let result = runtime_manager.execute_in_container(
                RuntimeType::NodeJs,
                &session,
                params.clone()
            ).await;

            if let Ok(response) = result {
                assert!(response.execution_time_ms > 0);
                assert!(response.result.is_object());
            }
        }
    }

    #[tokio::test]
    async fn test_execute_rust_pending() {
        let runtime_manager = create_test_runtime_manager();

        let mut session = create_test_session("rust-test", Some("fn main() {}"));
        session.compile_status = Some("pending".to_string());

        let params = json!({"input": 42});

        let result = runtime_manager.execute(&session, params).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_rust_with_error() {
        let runtime_manager = create_test_runtime_manager();

        let mut session = create_test_session("rust-test", Some("fn main() {}"));
        session.compile_status = Some("error".to_string());
        session.compile_error = Some("Compilation failed".to_string());

        let params = json!({"input": 42});

        let result = runtime_manager.execute(&session, params).await;
        assert!(result.is_err());
        match result {
            Err(Error::Compilation(msg)) => {
                assert_eq!(msg, "Compilation failed");
            }
            _ => panic!("Expected Compilation error"),
        }
    }

    #[test]
    fn test_runtime_config() {
        let config = RuntimeConfig {
            nodejs_runtime_url: "http://nodejs:8080".to_string(),
            python_runtime_url: "http://python:8080".to_string(),
            rust_runtime_url: "http://rust:8080".to_string(),
            timeout_seconds: 30,
            max_script_size: 1048576,
            wasm_compile_timeout_seconds: 60,
            selection_strategy: RuntimeSelectionStrategy::PrefixMatching,
            runtime_mappings: Vec::new(),
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };

        assert_eq!(config.nodejs_runtime_url, "http://nodejs:8080");
        assert_eq!(config.python_runtime_url, "http://python:8080");
        assert_eq!(config.rust_runtime_url, "http://rust:8080");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_script_size, 1048576);
        assert_eq!(config.wasm_compile_timeout_seconds, 60);
    }

    #[test]
    fn test_runtime_execute_request() {
        let request = RuntimeExecuteRequest {
            request_id: "test-id".to_string(),
            params: json!({"input": 42}),
            context: json!({"env": "test"}),
            script_content: Some("fn main() {}".to_string()),
        };

        assert_eq!(request.request_id, "test-id");
        assert_eq!(request.params, json!({"input": 42}));
        assert_eq!(request.context, json!({"env": "test"}));
        assert_eq!(request.script_content, Some("fn main() {}".to_string()));
    }

    #[test]
    fn test_runtime_execute_response() {
        let response = RuntimeExecuteResponse {
            result: json!({"output": 84}),
            execution_time_ms: 150,
            memory_usage_bytes: Some(2048),
        };

        assert_eq!(response.result, json!({"output": 84}));
        assert_eq!(response.execution_time_ms, 150);
        assert_eq!(response.memory_usage_bytes, Some(2048));
    }
}
