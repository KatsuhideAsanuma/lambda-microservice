use crate::{
    error::{Error, Result},
    runtime::{RuntimeExecuteResponse, RuntimeType},
    session::Session,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFaaSRequest {
    pub request_id: String,
    pub params: serde_json::Value,
    pub context: serde_json::Value,
    pub script_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFaaSResponse {
    pub result: serde_json::Value,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
}

pub struct OpenFaaSClient {
    client: Client,
    gateway_url: String,
    timeout: Duration,
}

impl OpenFaaSClient {
    pub fn new(gateway_url: &str, timeout_seconds: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            gateway_url: gateway_url.to_string(),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    pub async fn invoke_function(
        &self,
        function_name: &str,
        session: &Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        let url = format!("{}/function/{}/execute", self.gateway_url, function_name);
        debug!("Invoking OpenFaaS function: {}", url);

        let request_body = OpenFaaSRequest {
            request_id: session.request_id.clone(),
            params,
            context: session.context.clone(),
            script_content: session.script_content.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                error!("Error invoking OpenFaaS function: {}", e);
                Error::Runtime(format!("Failed to call OpenFaaS function: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "OpenFaaS function returned error status {}: {}",
                status, error_text
            );
            return Err(Error::Runtime(format!(
                "OpenFaaS function returned error status {}: {}",
                status, error_text
            )));
        }

        let openfaas_response: OpenFaaSResponse = response.json().await.map_err(|e| {
            error!("Error parsing OpenFaaS response: {}", e);
            Error::Runtime(format!("Failed to parse OpenFaaS response: {}", e))
        })?;

        info!(
            "OpenFaaS function executed in {}ms",
            openfaas_response.execution_time_ms
        );

        Ok(RuntimeExecuteResponse {
            result: openfaas_response.result,
            execution_time_ms: openfaas_response.execution_time_ms,
            memory_usage_bytes: openfaas_response.memory_usage_bytes,
        })
    }

    pub fn get_function_name_for_runtime(&self, runtime_type: RuntimeType) -> String {
        match runtime_type {
            RuntimeType::NodeJs => "nodejs-runtime".to_string(),
            RuntimeType::Python => "python-runtime".to_string(),
            RuntimeType::Rust => "rust-runtime".to_string(),
        }
    }
}
