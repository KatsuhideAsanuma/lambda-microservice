use crate::{
    api::RuntimeManagerTrait,
    error::{Error, Result},
    // kubernetes::{KubernetesClient, KubernetesClientTrait}, // TEMPORARILY DISABLED
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
// use wasmtime::Engine; // TEMPORARILY DISABLED
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
    // DynamicDiscovery, // TEMPORARILY DISABLED
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
    // pub wasm_compile_timeout_seconds: u64, // TEMPORARILY DISABLED
    pub selection_strategy: RuntimeSelectionStrategy,
    pub runtime_mappings: Vec<RuntimeMapping>,
    // pub kubernetes_namespace: Option<String>, // TEMPORARILY DISABLED
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
    // #[allow(dead_code)]
    // wasm_engine: Engine, // TEMPORARILY DISABLED
    openfaas_client: Option<OpenFaaSClient>,
    redis_client: Option<crate::cache::RedisClient<crate::cache::RedisPool>>,
    // kubernetes_client: Option<Box<dyn KubernetesClientTrait>>, // TEMPORARILY DISABLED
    protocol_factory: Arc<ProtocolFactory>,
}

impl<D: DbPoolTrait> RuntimeManager<D> {
    pub async fn new(config: &crate::config::RuntimeConfig, db_pool: D) -> Result<Self> {
        let selection_strategy = match config.selection_strategy.as_deref() {
            Some("config") => RuntimeSelectionStrategy::ConfigurationBased,
            // Some("discovery") => RuntimeSelectionStrategy::DynamicDiscovery, // TEMPORARILY DISABLED
            _ => RuntimeSelectionStrategy::PrefixMatching,
        };

        let mut runtime_mappings = Vec::new();
        if let Some(mappings_file) = &config.runtime_mappings_file {
            if let Ok(file_content) = tokio::fs::read_to_string(mappings_file).await {
                if let Ok(mappings) = serde_json::from_str::<Vec<RuntimeMapping>>(&file_content) {
                    runtime_mappings = mappings;
                    info!(
                        "Loaded {} runtime mappings from {}",
                        runtime_mappings.len(),
                        mappings_file
                    );
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
            // wasm_compile_timeout_seconds: config.wasm_compile_timeout_seconds, // TEMPORARILY DISABLED
            selection_strategy: selection_strategy.clone(),
            runtime_mappings,
            // kubernetes_namespace: config.kubernetes_namespace.clone(), // TEMPORARILY DISABLED
            redis_url: config.redis_url.clone(),
            cache_ttl_seconds: Some(config.cache_ttl_seconds.unwrap_or(3600)),
            runtime_max_retries: config.runtime_max_retries,
        };

        // let wasm_engine = Engine::default(); // TEMPORARILY DISABLED

        let openfaas_client = Some(OpenFaaSClient::new(
            &config.openfaas_gateway_url,
            config.runtime_timeout_seconds,
        ));

        let redis_client = if let Some(redis_url) = &config.redis_url {
            match crate::cache::RedisClient::<crate::cache::RedisPool>::new(redis_url).await {
                Ok(client) => {
                    info!("Connected to Redis at {}", redis_url);
                    Some(client)
                }
                Err(e) => {
                    warn!("Failed to connect to Redis at {}: {}", redis_url, e);
                    None
                }
            }
        } else {
            None
        };

        // Kubernetes client temporarily disabled
        // let kubernetes_client = if selection_strategy == RuntimeSelectionStrategy::DynamicDiscovery {
        //     if let Some(namespace) = &config.kubernetes_namespace {
        //         match KubernetesClient::new(
        //             namespace,
        //             config.cache_ttl_seconds.unwrap_or(3600),
        //         ).await {
        //             Ok(client) => {
        //                 info!("Connected to Kubernetes API for namespace {}", namespace);
        //                 Some(Box::new(client) as Box<dyn KubernetesClientTrait>)
        //             },
        //             Err(e) => {
        //                 warn!("Failed to connect to Kubernetes API: {}", e);
        //                 None
        //             }
        //         }
        //     } else {
        //         warn!("Dynamic discovery selected but no Kubernetes namespace configured");
        //         None
        //     }
        // } else {
        //     None
        // };

        Ok(Self {
            config: runtime_config,
            db_pool,
            // wasm_engine, // TEMPORARILY DISABLED
            openfaas_client,
            redis_client,
            // kubernetes_client, // TEMPORARILY DISABLED
            protocol_factory: Arc::new(ProtocolFactory::new()),
        })
    }

    pub async fn get_runtime_type(&self, language_title: &str) -> Result<RuntimeType> {
        match self.config.selection_strategy {
            RuntimeSelectionStrategy::PrefixMatching => {
                RuntimeType::from_language_title(language_title)
            }
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
                            }
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
            }
            // DynamicDiscovery temporarily disabled
            // RuntimeSelectionStrategy::DynamicDiscovery => {
            //     if let Some(namespace) = &self.config.kubernetes_namespace {
            //         info!("Dynamic discovery requested for '{}' in namespace '{}'",
            //               language_title, namespace);

            //         if let Some(kubernetes_client) = &self.kubernetes_client {
            //             match kubernetes_client.get_runtime_type_for_language(language_title).await {
            //                 Ok(runtime_type) => {
            //                     info!("Found runtime type {:?} for '{}' using Kubernetes discovery",
            //                           runtime_type, language_title);
            //                     return Ok(runtime_type);
            //                 },
            //                 Err(e) => {
            //                     warn!("Kubernetes discovery failed: {}, falling back to prefix matching", e);
            //                 }
            //             }
            //         } else {
            //             warn!("Kubernetes client not initialized, falling back to prefix matching");
            //         }
            //     } else {
            //         warn!("Dynamic discovery selected but no Kubernetes namespace configured");
            //     }

            //     RuntimeType::from_language_title(language_title)
            // }
        }
    }
}

#[async_trait]
impl<D: DbPoolTrait + Send + Sync> RuntimeManagerTrait for RuntimeManager<D> {
    async fn execute<'a>(
        &'a self,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let runtime_type = self.get_runtime_type(&session.language_title).await?;

        // All runtimes now use container execution (WebAssembly temporarily disabled)
        self.execute_in_container(runtime_type, session, params)
            .await
    }

    // WebAssembly compilation temporarily disabled
    async fn compile_rust_script<'a>(&'a self, _session: &'a Session) -> Result<Vec<u8>> {
        Err(Error::Runtime(
            "WebAssembly compilation is temporarily disabled".to_string(),
        ))
    }

    // WebAssembly compilation temporarily disabled
    async fn compile_with_wasmtime<'a>(
        &'a self,
        _script_content: &'a str,
        _memory_limit_bytes: u64,
    ) -> Result<Vec<u8>> {
        Err(Error::Runtime(
            "WebAssembly compilation is temporarily disabled".to_string(),
        ))
    }

    // WebAssembly execution temporarily disabled
    async fn execute_wasm<'a>(
        &'a self,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        Err(Error::Runtime(
            "WebAssembly execution is temporarily disabled".to_string(),
        ))
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

            match openfaas_client
                .invoke_function(&function_name, session, params.clone())
                .await
            {
                Ok(response) => {
                    info!("Successfully executed via OpenFaaS: {}", function_name);
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        "OpenFaaS execution failed, falling back to direct container: {}",
                        e
                    );
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

        use tokio_retry::strategy::{jitter, ExponentialBackoff};
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
        })
        .await?;

        let runtime_response = response
            .json::<RuntimeExecuteResponse>()
            .await
            .map_err(Error::from)?;

        Ok(runtime_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SessionStatus};
    use chrono::{Duration as ChronoDuration, Utc};
    use serde_json::json;

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
            // wasm_compile_timeout_seconds: 60, // TEMPORARILY DISABLED
            selection_strategy: RuntimeSelectionStrategy::PrefixMatching,
            runtime_mappings: Vec::new(),
            // kubernetes_namespace: None, // TEMPORARILY DISABLED
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };

        let db_pool = MockPostgresPool::new();
        // let wasm_engine = Engine::default(); // TEMPORARILY DISABLED

        RuntimeManager {
            config,
            db_pool,
            // wasm_engine, // TEMPORARILY DISABLED
            openfaas_client: None,
            redis_client: None,
            // kubernetes_client: None, // TEMPORARILY DISABLED
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

    #[test]
    fn test_runtime_type_get_runtime_url() {
        let config = RuntimeConfig {
            nodejs_runtime_url: "http://nodejs:8080".to_string(),
            python_runtime_url: "http://python:8080".to_string(),
            rust_runtime_url: "http://rust:8080".to_string(),
            timeout_seconds: 30,
            max_script_size: 1048576,
            // wasm_compile_timeout_seconds: 60, // TEMPORARILY DISABLED
            selection_strategy: RuntimeSelectionStrategy::PrefixMatching,
            runtime_mappings: Vec::new(),
            // kubernetes_namespace: None, // TEMPORARILY DISABLED
            redis_url: None,
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        };

        assert_eq!(
            RuntimeType::NodeJs.get_runtime_url(&config),
            "http://nodejs:8080"
        );
        assert_eq!(
            RuntimeType::Python.get_runtime_url(&config),
            "http://python:8080"
        );
        assert_eq!(
            RuntimeType::Rust.get_runtime_url(&config),
            "http://rust:8080"
        );
    }

    #[tokio::test]
    async fn test_compile_rust_script_disabled() {
        let runtime_manager = create_test_runtime_manager();
        let session = create_test_session("rust-test", Some("fn main() {}"));

        let result = runtime_manager.compile_rust_script(&session).await;
        assert!(result.is_err());
        match result {
            Err(Error::Runtime(msg)) => {
                assert!(msg.contains("WebAssembly compilation is temporarily disabled"));
            }
            _ => panic!("Expected Runtime error"),
        }
    }

    #[tokio::test]
    async fn test_execute_wasm_disabled() {
        let runtime_manager = create_test_runtime_manager();
        let session = create_test_session("rust-test", Some("fn main() {}"));
        let params = json!({"input": 42});

        let result = runtime_manager.execute_wasm(&session, params).await;
        assert!(result.is_err());
        match result {
            Err(Error::Runtime(msg)) => {
                assert!(msg.contains("WebAssembly execution is temporarily disabled"));
            }
            _ => panic!("Expected Runtime error"),
        }
    }
}
