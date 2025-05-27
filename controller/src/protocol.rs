use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use async_trait::async_trait;
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Json,
    Grpc,
}

#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    async fn send_request(&self, url: &str, payload: &[u8], timeout_ms: u64) -> Result<Vec<u8>>;
}

pub struct JsonProtocolAdapter {
    client: reqwest::Client,
}

impl JsonProtocolAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ProtocolAdapter for JsonProtocolAdapter {
    async fn send_request(&self, url: &str, payload: &[u8], timeout_ms: u64) -> Result<Vec<u8>> {
        debug!("Sending JSON request to {}", url);
        
        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .body(payload.to_vec())
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| Error::Runtime(format!("Failed to send JSON request: {}", e)))?;
        
        if response.status().is_success() {
            response.bytes().await
                .map(|b| b.to_vec())
                .map_err(|e| Error::Runtime(format!("Failed to read JSON response: {}", e)))
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(Error::Runtime(format!("Runtime returned error status {}: {}", status, error_text)))
        }
    }
}

pub mod grpc;
use grpc::GrpcProtocolAdapter;

#[cfg(test)]
pub mod tests;

pub struct ProtocolFactory {
    json_adapter: Arc<JsonProtocolAdapter>,
    grpc_adapter: Arc<GrpcProtocolAdapter>,
}

impl ProtocolFactory {
    pub fn new() -> Self {
        Self {
            json_adapter: Arc::new(JsonProtocolAdapter::new()),
            grpc_adapter: Arc::new(GrpcProtocolAdapter::new()),
        }
    }
    
    pub fn get_adapter(&self, protocol_type: ProtocolType) -> Result<Arc<dyn ProtocolAdapter>> {
        match protocol_type {
            ProtocolType::Json => Ok(self.json_adapter.clone()),
            ProtocolType::Grpc => Ok(self.grpc_adapter.clone()),
        }
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    
    #[test]
    fn test_protocol_factory_creation() {
        let factory = ProtocolFactory::new();
        assert!(factory.get_adapter(ProtocolType::Json).is_ok());
        assert!(factory.get_adapter(ProtocolType::Grpc).is_ok());
    }
}
