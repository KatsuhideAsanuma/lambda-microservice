use crate::error::{Error, Result};
use crate::protocol::ProtocolAdapter;
use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
    collections::HashMap,
};
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, warn, info};
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod runtime {
    tonic::include_proto!("runtime");
}

use runtime::{
    runtime_service_client::RuntimeServiceClient,
    ExecuteRequest, ExecuteResponse, 
    InitializeRequest, InitializeResponse,
    HealthCheckRequest, HealthCheckResponse,
    MetricsRequest, MetricsResponse,
    LogsRequest, LogsResponse,
    ConfigRequest, ConfigResponse,
};

pub struct CircuitBreakerConfig {
    failure_threshold: usize,
    reset_timeout: Duration,
}

enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Mutex<CircuitState>,
    failures: AtomicUsize,
    last_failure: Mutex<std::time::Instant>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed),
            failures: AtomicUsize::new(0),
            last_failure: Mutex::new(std::time::Instant::now()),
            config,
        }
    }
    
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut state = self.state.lock().unwrap();
        *state = CircuitState::Closed;
    }
    
    pub fn record_failure(&self) -> bool {
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure.lock().unwrap() = std::time::Instant::now();
        
        let mut state = self.state.lock().unwrap();
        
        match *state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    info!("Circuit breaker opened after {} failures", failures);
                    *state = CircuitState::Open;
                    return false;
                }
                true
            },
            CircuitState::HalfOpen => {
                info!("Circuit breaker opened after failure in half-open state");
                *state = CircuitState::Open;
                false
            },
            CircuitState::Open => false,
        }
    }
    
    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = *self.last_failure.lock().unwrap();
                if last_failure.elapsed() >= self.config.reset_timeout {
                    info!("Circuit breaker half-open after timeout");
                    *state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => true,
        }
    }
}

struct TimeoutConfig {
    execute: Duration,
    initialize: Duration,
    health_check: Duration,
    metrics: Duration,
    logs: Duration,
    config: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            execute: Duration::from_secs(30),
            initialize: Duration::from_secs(60),
            health_check: Duration::from_secs(5),
            metrics: Duration::from_secs(10),
            logs: Duration::from_secs(15),
            config: Duration::from_secs(10),
        }
    }
}

pub struct GrpcProtocolAdapter {
    client_cache: Mutex<HashMap<String, RuntimeServiceClient<Channel>>>,
    circuit_breakers: Mutex<HashMap<String, Arc<CircuitBreaker>>>,
    timeout_config: TimeoutConfig,
}

impl GrpcProtocolAdapter {
    pub fn new() -> Self {
        Self {
            client_cache: Mutex::new(HashMap::new()),
            circuit_breakers: Mutex::new(HashMap::new()),
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    pub fn get_circuit_breaker(&self, url: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.lock().unwrap();
        
        breakers.entry(url.to_string()).or_insert_with(|| {
            Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: Duration::from_secs(30),
            }))
        }).clone()
    }
    
    pub fn get_timeout(&self, operation: &str) -> Duration {
        match operation {
            "execute" => self.timeout_config.execute,
            "initialize" => self.timeout_config.initialize,
            "health_check" => self.timeout_config.health_check,
            "metrics" => self.timeout_config.metrics,
            "logs" => self.timeout_config.logs,
            "config" => self.timeout_config.config,
            _ => Duration::from_secs(10), // Default timeout
        }
    }
    
    async fn get_client(&self, url: &str) -> Result<RuntimeServiceClient<Channel>> {
        {
            let cache = self.client_cache.lock().unwrap();
            if let Some(client) = cache.get(url) {
                return Ok(client.clone());
            }
        }
        
        let endpoint = Endpoint::from_shared(url.to_string())
            .map_err(|e| Error::Runtime(format!("Invalid gRPC endpoint: {}", e)))?
            .connect_timeout(Duration::from_secs(5));
            
        let client = RuntimeServiceClient::connect(endpoint)
            .await
            .map_err(|e| Error::Runtime(format!("Failed to connect to gRPC endpoint: {}", e)))?;
            
        {
            let mut cache = self.client_cache.lock().unwrap();
            cache.insert(url.to_string(), client.clone());
        }
        
        Ok(client)
    }
    
    pub async fn with_retry<F, Fut, T>(&self, url: &str, _operation: &str, f: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        let circuit_breaker = self.get_circuit_breaker(url);
        
        if !circuit_breaker.allow_request() {
            return Err(Error::Runtime("Circuit breaker is open".to_string()));
        }
        
        let max_retries = 3;
        let base_delay = Duration::from_millis(100);
        
        for attempt in 0..=max_retries {
            match f().await {
                Ok(result) => {
                    circuit_breaker.record_success();
                    return Ok(result);
                },
                Err(e) if attempt < max_retries => {
                    if !circuit_breaker.record_failure() {
                        return Err(Error::Runtime(format!("Circuit breaker opened after failure: {}", e)));
                    }
                    
                    let delay = base_delay * (2_u32.pow(attempt as u32));
                    let jitter = (rand::random::<f64>() * 0.2 - 0.1) * delay.as_millis() as f64;
                    let delay = Duration::from_millis((delay.as_millis() as f64 + jitter) as u64);
                    
                    warn!("Attempt {} failed: {}. Retrying after {:?}", attempt + 1, e, delay);
                    tokio::time::sleep(delay).await;
                },
                Err(e) => {
                    circuit_breaker.record_failure();
                    return Err(e);
                }
            }
        }
        
        Err(Error::Runtime("Failed to execute gRPC request after retries".to_string()))
    }
    
    pub fn degraded_operation(&self, error: &Error, operation: &str) -> Result<Vec<u8>> {
        warn!("Using degraded operation for {}: {}", operation, error);
        
        match operation {
            "execute" => {
                let response = serde_json::json!({
                    "result": "Degraded operation: unable to execute normally",
                    "execution_time_ms": 0,
                    "degraded": true
                });
                
                Ok(serde_json::to_vec(&response)?)
            },
            "health_check" => {
                let response = serde_json::json!({
                    "status": "degraded",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                
                Ok(serde_json::to_vec(&response)?)
            },
            _ => Err(Error::Runtime(format!("No degraded operation available for {}", operation))),
        }
    }
}

#[async_trait]
impl ProtocolAdapter for GrpcProtocolAdapter {
    async fn send_request(&self, url: &str, payload: &[u8], _timeout_ms: u64) -> Result<Vec<u8>> {
        debug!("Sending gRPC request to {}", url);
        
        let payload_str = std::str::from_utf8(payload)
            .map_err(|e| Error::Runtime(format!("Invalid UTF-8 in payload: {}", e)))?;
            
        let payload_json: serde_json::Value = serde_json::from_str(payload_str)
            .map_err(|e| Error::Runtime(format!("Invalid JSON in payload: {}", e)))?;
            
        let request_type = payload_json["request_type"]
            .as_str()
            .unwrap_or("execute");
            
        let timeout = self.get_timeout(request_type);
        
        let result = self.with_retry(url, request_type, || async {
            let mut client = self.get_client(url).await?;
            
            match request_type {
                "execute" => {
                    let request = ExecuteRequest {
                        request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
                        params: payload_json["params"].to_string(),
                        context: payload_json["context"].to_string(),
                        script_content: payload_json["script_content"].as_str().map(|s| s.to_string()),
                    };
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.execute(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC execute request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC execute request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "result": response.get_ref().result,
                        "execution_time_ms": response.get_ref().execution_time_ms,
                        "memory_usage_bytes": response.get_ref().memory_usage_bytes,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize execute response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                "initialize" => {
                    let request = InitializeRequest {
                        request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
                        context: payload_json["context"].to_string(),
                        script_content: payload_json["script_content"].as_str().unwrap_or("").to_string(),
                    };
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.initialize(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC initialize request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC initialize request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "request_id": response.get_ref().request_id,
                        "success": response.get_ref().success,
                        "error": response.get_ref().error,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize initialize response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                "health_check" => {
                    let request = HealthCheckRequest {};
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.health_check(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC health check request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC health check request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "status": response.get_ref().status,
                        "timestamp": response.get_ref().timestamp,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize health check response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                "metrics" => {
                    let request = MetricsRequest {
                        request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
                        metric_name: payload_json["metric_name"].as_str().map(|s| s.to_string()),
                        time_range: payload_json["time_range"].as_str().map(|s| s.to_string()),
                    };
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.get_metrics(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC metrics request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC metrics request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "request_id": response.get_ref().request_id,
                        "metrics": response.get_ref().metrics,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize metrics response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                "logs" => {
                    let request = LogsRequest {
                        request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
                        log_level: payload_json["log_level"].as_str().map(|s| s.to_string()),
                        time_range: payload_json["time_range"].as_str().map(|s| s.to_string()),
                        limit: payload_json["limit"].as_u64().map(|n| n as u32),
                        offset: payload_json["offset"].as_u64().map(|n| n as u32),
                    };
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.get_logs(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC logs request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC logs request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "request_id": response.get_ref().request_id,
                        "logs": response.get_ref().logs,
                        "total_count": response.get_ref().total_count,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize logs response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                "config" => {
                    let request = ConfigRequest {
                        request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
                        config: payload_json["config"].to_string(),
                    };
                    
                    let response = tokio::time::timeout(
                        timeout,
                        client.update_config(request)
                    ).await
                     .map_err(|_| Error::Runtime("gRPC config request timed out".to_string()))?
                     .map_err(|e| Error::Runtime(format!("gRPC config request failed: {}", e)))?;
                     
                    let response_json = serde_json::json!({
                        "request_id": response.get_ref().request_id,
                        "success": response.get_ref().success,
                        "error": response.get_ref().error,
                        "current_config": response.get_ref().current_config,
                    });
                    
                    let response_bytes = serde_json::to_vec(&response_json)
                        .map_err(|e| Error::Runtime(format!("Failed to serialize config response: {}", e)))?;
                        
                    Ok(response_bytes)
                },
                _ => Err(Error::Runtime(format!("Unknown request type: {}", request_type))),
            }
        }).await;
        
        match result {
            Ok(response) => Ok(response),
            Err(e) => self.degraded_operation(&e, request_type),
        }
    }
}
