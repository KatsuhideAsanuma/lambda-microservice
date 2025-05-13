
use crate::{
    database::PostgresPool,
    error::{Error, Result},
    session::Session,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use wasmtime::{Engine, Instance, Module, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    NodeJs,
    Python,
    Rust,
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

    pub fn get_runtime_url(&self, config: &RuntimeConfig) -> &str {
        match self {
            Self::NodeJs => &config.nodejs_runtime_url,
            Self::Python => &config.python_runtime_url,
            Self::Rust => &config.rust_runtime_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub nodejs_runtime_url: String,
    pub python_runtime_url: String,
    pub rust_runtime_url: String,
    pub timeout_seconds: u64,
    pub max_script_size: usize,
    pub wasm_compile_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeExecuteRequest {
    pub request_id: String,
    pub params: serde_json::Value,
    pub context: serde_json::Value,
    pub script_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeExecuteResponse {
    pub result: serde_json::Value,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
}

pub struct RuntimeManager {
    config: RuntimeConfig,
    db_pool: PostgresPool,
    wasm_engine: Engine,
}

impl RuntimeManager {
    pub async fn new(config: &crate::config::RuntimeConfig, db_pool: PostgresPool) -> Result<Self> {
        let runtime_config = RuntimeConfig {
            nodejs_runtime_url: config.nodejs_runtime_url.clone(),
            python_runtime_url: config.python_runtime_url.clone(),
            rust_runtime_url: config.rust_runtime_url.clone(),
            timeout_seconds: config.runtime_timeout_seconds,
            max_script_size: config.max_script_size,
            wasm_compile_timeout_seconds: config.wasm_compile_timeout_seconds,
        };

        let wasm_engine = Engine::default();

        Ok(Self {
            config: runtime_config,
            db_pool,
            wasm_engine,
        })
    }

    pub async fn execute(
        &self,
        session: &Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let runtime_type = RuntimeType::from_language_title(&session.language_title)?;

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

    async fn compile_rust_script(&self, session: &Session) -> Result<Vec<u8>> {

        let script_content = session
            .script_content
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Script content is required".to_string()))?;

        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, // WebAssembly header
        ])
    }

    async fn execute_wasm(
        &self,
        session: &Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {

        let compiled_artifact = session
            .compiled_artifact
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Compiled artifact is required".to_string()))?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(RuntimeExecuteResponse {
            result: serde_json::json!({
                "result": "Simulated WebAssembly execution result",
                "params": params,
            }),
            execution_time_ms: 100,
            memory_usage_bytes: Some(1024 * 1024), // 1MB
        })
    }

    async fn execute_in_container(
        &self,
        runtime_type: RuntimeType,
        session: &Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let runtime_url = runtime_type.get_runtime_url(&self.config);

        let request = RuntimeExecuteRequest {
            request_id: session.request_id.clone(),
            params,
            context: session.context.clone(),
            script_content: session.script_content.clone(),
        };

        let client = reqwest::Client::new();
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            client
                .post(format!("{}/execute", runtime_url))
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| Error::Runtime("Runtime execution timed out".to_string()))??;

        let runtime_response = response
            .json::<RuntimeExecuteResponse>()
            .await
            .map_err(Error::from)?;

        Ok(runtime_response)
    }
}
