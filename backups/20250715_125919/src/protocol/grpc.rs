use crate::error::{Error, Result};
use crate::protocol::ProtocolAdapter;
use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
    collections::HashMap,
};
// use tonic::transport::{Channel, Endpoint}; // TEMPORARILY DISABLED
use tracing::{debug, error, warn, info};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    Execute,
    Initialize,
    HealthCheck,
    Metrics,
    Logs,
    Config,
}

impl RequestType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "execute" => Some(RequestType::Execute),
            "initialize" => Some(RequestType::Initialize),
            "health_check" => Some(RequestType::HealthCheck),
            "metrics" => Some(RequestType::Metrics),
            "logs" => Some(RequestType::Logs),
            "config" => Some(RequestType::Config),
            _ => None,
        }
    }
}

// gRPC proto compilation temporarily disabled
// pub mod runtime {
//     tonic::include_proto!("runtime");
// }

// use runtime::{
//     runtime_service_client::RuntimeServiceClient,
//     ExecuteRequest, ExecuteResponse, 
//     InitializeRequest, InitializeResponse,
//     HealthCheckRequest, HealthCheckResponse,
//     MetricsRequest, MetricsResponse,
//     LogsRequest, LogsResponse,
//     ConfigRequest, ConfigResponse,
// };

#[async_trait]
pub trait GrpcClient: Send + Sync {
    async fn send_execute_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
    async fn send_initialize_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
    async fn send_health_check_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
    async fn send_metrics_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
    async fn send_logs_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
    async fn send_config_request(&self, payload: String, timeout_ms: u64) -> Result<String>;
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub reset_timeout: Duration,
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
    // client_cache: Mutex<HashMap<String, RuntimeServiceClient<Channel>>>, // TEMPORARILY DISABLED
    circuit_breakers: Mutex<HashMap<String, Arc<CircuitBreaker>>>,
    timeout_config: TimeoutConfig,
}

impl GrpcProtocolAdapter {
    pub async fn handle_request(&self, client: Arc<dyn GrpcClient>, request_type: &str, payload: &str, timeout_ms: u64) -> Result<String> {
        match RequestType::from_str(request_type) {
            Some(RequestType::Execute) => client.send_execute_request(payload.to_string(), timeout_ms).await,
            Some(RequestType::Initialize) => client.send_initialize_request(payload.to_string(), timeout_ms).await,
            Some(RequestType::HealthCheck) => client.send_health_check_request(payload.to_string(), timeout_ms).await,
            Some(RequestType::Metrics) => client.send_metrics_request(payload.to_string(), timeout_ms).await,
            Some(RequestType::Logs) => client.send_logs_request(payload.to_string(), timeout_ms).await,
            Some(RequestType::Config) => client.send_config_request(payload.to_string(), timeout_ms).await,
            None => Err(Error::Runtime(format!("Unknown request type: {}", request_type))),
        }
    }
    
    pub fn new() -> Self {
        Self {
            // client_cache: Mutex::new(HashMap::new()), // TEMPORARILY DISABLED
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
    
    // gRPC client temporarily disabled
    // pub async fn get_client(&self, url: &str) -> Result<RuntimeServiceClient<Channel>> {
    //     {
    //         let cache = self.client_cache.lock().unwrap();
    //         if let Some(client) = cache.get(url) {
    //             return Ok(client.clone());
    //         }
    //     }
        
    //     let endpoint = Endpoint::from_shared(url.to_string())
    //         .map_err(|e| Error::Runtime(format!("Invalid gRPC endpoint: {}", e)))?
    //         .connect_timeout(Duration::from_secs(5));
            
    //     let client = RuntimeServiceClient::connect(endpoint)
    //         .await
    //         .map_err(|e| Error::Runtime(format!("Failed to connect to gRPC endpoint: {}", e)))?;
            
    //     {
    //         let mut cache = self.client_cache.lock().unwrap();
    //         cache.insert(url.to_string(), client.clone());
    //     }
        
    //     Ok(client)
    // }
    
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
    async fn send_request(&self, _url: &str, _payload: &[u8], _timeout_ms: u64) -> Result<Vec<u8>> {
        // gRPC functionality temporarily disabled
        Err(Error::Runtime("gRPC protocol is temporarily disabled".to_string()))
        
        // Original implementation commented out:
        // debug!("Sending gRPC request to {}", url);
        
        // let payload_str = std::str::from_utf8(payload)
        //     .map_err(|e| Error::Runtime(format!("Invalid UTF-8 in payload: {}", e)))?;
            
        // let payload_json: serde_json::Value = serde_json::from_str(payload_str)
        //     .map_err(|e| Error::Runtime(format!("Invalid JSON in payload: {}", e)))?;
            
        // let request_type = payload_json["request_type"]
        //     .as_str()
        //     .unwrap_or("execute");
            
        // let timeout = self.get_timeout(request_type);
        
        // let result = self.with_retry(url, request_type, || async {
        //     let mut client = self.get_client(url).await?;
            
        //     match request_type {
        //         "execute" => {
        //             let request = ExecuteRequest {
        //                 request_id: payload_json["request_id"].as_str().unwrap_or("").to_string(),
        //                 params: payload_json["params"].to_string(),
        //                 context: payload_json["context"].to_string(),
        //                 script_content: payload_json["script_content"].as_str().map(|s| s.to_string()),
        //             };
                    
        //             let response = tokio::time::timeout(
        //                 timeout,
        //                 client.execute(request)
        //             ).await
        //              .map_err(|_| Error::Runtime("gRPC execute request timed out".to_string()))?
        //              .map_err(|e| Error::Runtime(format!("gRPC execute request failed: {}", e)))?;
                     
        //             let response_json = serde_json::json!({
        //                 "result": response.get_ref().result,
        //                 "execution_time_ms": response.get_ref().execution_time_ms,
        //                 "memory_usage_bytes": response.get_ref().memory_usage_bytes,
        //             });
                    
        //             let response_bytes = serde_json::to_vec(&response_json)
        //                 .map_err(|e| Error::Runtime(format!("Failed to serialize execute response: {}", e)))?;
                        
        //             Ok(response_bytes)
        //         },
        //         // ... other request types ...
        //         _ => Err(Error::Runtime(format!("Unknown request type: {}", request_type))),
        //     }
        // }).await;
        
        // match result {
        //     Ok(response) => Ok(response),
        //     Err(e) => self.degraded_operation(&e, request_type),
        // }
    }
}
