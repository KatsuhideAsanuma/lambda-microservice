use crate::error::{Error, Result};
use crate::protocol::grpc::{CircuitBreaker, CircuitBreakerConfig, GrpcProtocolAdapter, GrpcClient, RequestType};
use std::time::Duration;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;

#[test]
fn test_grpc_protocol_adapter_new() {
    let adapter = GrpcProtocolAdapter::new();
    assert!(adapter.get_timeout("execute") > Duration::from_secs(0));
    assert!(adapter.get_timeout("initialize") > Duration::from_secs(0));
    assert!(adapter.get_timeout("health_check") > Duration::from_secs(0));
    assert!(adapter.get_timeout("metrics") > Duration::from_secs(0));
    assert!(adapter.get_timeout("logs") > Duration::from_secs(0));
    assert!(adapter.get_timeout("config") > Duration::from_secs(0));
    assert!(adapter.get_timeout("unknown") > Duration::from_secs(0));
}

#[test]
fn test_get_circuit_breaker() {
    let adapter = GrpcProtocolAdapter::new();
    
    let breaker1 = adapter.get_circuit_breaker("http://test:8080");
    let breaker2 = adapter.get_circuit_breaker("http://test:8080");
    
    assert!(Arc::ptr_eq(&breaker1, &breaker2));
    
    let breaker3 = adapter.get_circuit_breaker("http://another:8080");
    assert!(!Arc::ptr_eq(&breaker1, &breaker3));
    
    assert!(breaker1.allow_request());
}

#[test]
fn test_get_timeout() {
    let adapter = GrpcProtocolAdapter::new();
    
    assert_eq!(adapter.get_timeout("execute"), Duration::from_secs(30));
    assert_eq!(adapter.get_timeout("initialize"), Duration::from_secs(60));
    assert_eq!(adapter.get_timeout("health_check"), Duration::from_secs(5));
    assert_eq!(adapter.get_timeout("metrics"), Duration::from_secs(10));
    assert_eq!(adapter.get_timeout("logs"), Duration::from_secs(15));
    assert_eq!(adapter.get_timeout("config"), Duration::from_secs(10));
    
    assert_eq!(adapter.get_timeout("unknown"), Duration::from_secs(10));
}

#[test]
fn test_circuit_breaker_config() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(30),
    };
    
    assert_eq!(config.failure_threshold, 3);
    assert_eq!(config.reset_timeout, Duration::from_secs(30));
}

#[test]
fn test_circuit_breaker_new() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(30),
    };
    
    let breaker = CircuitBreaker::new(config);
    assert!(breaker.allow_request());
}

#[test]
fn test_circuit_breaker_state_transitions() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        reset_timeout: Duration::from_millis(100),
    };
    
    let breaker = CircuitBreaker::new(config);
    
    assert!(breaker.allow_request());
    
    assert!(breaker.record_failure());
    assert!(breaker.allow_request());
    
    assert!(!breaker.record_failure());
    assert!(!breaker.allow_request());
    
    breaker.record_success();
    assert!(breaker.allow_request());
}

#[test]
fn test_grpc_protocol_adapter_degraded_operation() {
    let adapter = GrpcProtocolAdapter::new();
    let error = Error::Runtime("Test error".to_string());
    
    let execute_result = adapter.degraded_operation(&error, "execute");
    assert!(execute_result.is_ok());
    if let Ok(response) = execute_result {
        let response_json: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response_json["result"], "Degraded operation: unable to execute normally");
        assert_eq!(response_json["execution_time_ms"], 0);
        assert_eq!(response_json["degraded"], true);
    }
    
    let health_result = adapter.degraded_operation(&error, "health_check");
    assert!(health_result.is_ok());
    if let Ok(response) = health_result {
        let response_json: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response_json["status"], "degraded");
        assert!(response_json["timestamp"].is_string());
    }
    
    let unsupported_result = adapter.degraded_operation(&error, "unsupported");
    assert!(unsupported_result.is_err());
    if let Err(e) = unsupported_result {
        match e {
            Error::Runtime(msg) => {
                assert!(msg.contains("No degraded operation available"));
            },
            _ => panic!("Expected Runtime error"),
        }
    }
}

struct MockGrpcAdapter {
    failure_count: std::sync::atomic::AtomicUsize,
    max_failures: usize,
}

impl MockGrpcAdapter {
    fn new(max_failures: usize) -> Self {
        Self {
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            max_failures,
        }
    }
    
    async fn test_operation(&self) -> Result<String> {
        let count = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < self.max_failures {
            Err(Error::Runtime(format!("Simulated failure {}", count)))
        } else {
            Ok("Success".to_string())
        }
    }
}

#[derive(Clone)]
struct MockGrpcClientWithHistory {
    request_history: Arc<TokioMutex<Vec<(String, String, u64)>>>,
    response: String,
}

impl MockGrpcClientWithHistory {
    fn new(response: String) -> Self {
        Self {
            request_history: Arc::new(TokioMutex::new(Vec::new())),
            response,
        }
    }
    
    async fn get_request_history(&self) -> Vec<(String, String, u64)> {
        self.request_history.lock().await.clone()
    }
}

impl GrpcClient for MockGrpcClientWithHistory {
    async fn send_execute_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("execute".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
    
    async fn send_initialize_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("initialize".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
    
    async fn send_health_check_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("health_check".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
    
    async fn send_metrics_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("metrics".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
    
    async fn send_logs_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("logs".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
    
    async fn send_config_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("config".to_string(), payload, timeout_ms));
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn test_get_client() {
    let adapter = GrpcProtocolAdapter::new();
    
    let client1 = adapter.get_client("http://test:8080");
    let client2 = adapter.get_client("http://test:8080");
    
    assert!(Arc::ptr_eq(&client1, &client2));
    
    let client3 = adapter.get_client("http://another:8080");
    assert!(!Arc::ptr_eq(&client1, &client3));
}

#[tokio::test]
async fn test_send_request() {
    let adapter = GrpcProtocolAdapter::new();
    let mock_client = MockGrpcClientWithHistory::new("test response".to_string());
    
    let result = adapter.send_request(
        Arc::new(mock_client.clone()),
        "execute",
        "test payload",
        1000
    ).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test response");
    
    let history = mock_client.get_request_history().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].0, "execute");
    assert_eq!(history[0].1, "test payload");
    assert_eq!(history[0].2, 1000);
}

#[tokio::test]
async fn test_with_retry_success_after_failures() {
    let mock = MockGrpcAdapter::new(2);
    let adapter = GrpcProtocolAdapter::new();
    
    let result = adapter.with_retry("http://test:8080", "test", || async {
        mock.test_operation().await
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(mock.failure_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_with_retry_all_failures() {
    let mock = MockGrpcAdapter::new(10); // More than max retries
    let adapter = GrpcProtocolAdapter::new();
    
    let result = adapter.with_retry("http://test:8080", "test", || async {
        mock.test_operation().await
    }).await;
    
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            Error::Runtime(msg) => {
                assert!(msg.contains("Failed to execute") || msg.contains("after retries") || msg.contains("Circuit breaker"));
            },
            _ => panic!("Expected Runtime error"),
        }
    }
}

#[tokio::test]
async fn test_handle_request() {
    use crate::protocol::grpc::{GrpcClient, RequestType};
    
    let execute = RequestType::from_str("execute");
    assert_eq!(execute, Some(RequestType::Execute));
    
    let initialize = RequestType::from_str("initialize");
    assert_eq!(initialize, Some(RequestType::Initialize));
    
    let health_check = RequestType::from_str("health_check");
    assert_eq!(health_check, Some(RequestType::HealthCheck));
    
    let metrics = RequestType::from_str("metrics");
    assert_eq!(metrics, Some(RequestType::Metrics));
    
    let logs = RequestType::from_str("logs");
    assert_eq!(logs, Some(RequestType::Logs));
    
    let config = RequestType::from_str("config");
    assert_eq!(config, Some(RequestType::Config));
    
    let unknown = RequestType::from_str("unknown");
    assert_eq!(unknown, None);
    
    struct MockGrpcClient;
    
    impl GrpcClient for MockGrpcClient {
        async fn send_execute_request(&self, _: String, _: u64) -> Result<String> {
            Ok("execute response".to_string())
        }
        
        async fn send_initialize_request(&self, _: String, _: u64) -> Result<String> {
            Ok("initialize response".to_string())
        }
        
        async fn send_health_check_request(&self, _: String, _: u64) -> Result<String> {
            Ok("health response".to_string())
        }
        
        async fn send_metrics_request(&self, _: String, _: u64) -> Result<String> {
            Ok("metrics response".to_string())
        }
        
        async fn send_logs_request(&self, _: String, _: u64) -> Result<String> {
            Ok("logs response".to_string())
        }
        
        async fn send_config_request(&self, _: String, _: u64) -> Result<String> {
            Ok("config response".to_string())
        }
    }
    
    let adapter = GrpcProtocolAdapter::new();
    let client = Arc::new(MockGrpcClient);
    
    let result = adapter.handle_request(client.clone(), "execute", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "execute response");
    
    let result = adapter.handle_request(client.clone(), "initialize", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "initialize response");
    
    let result = adapter.handle_request(client.clone(), "health_check", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "health response");
    
    let result = adapter.handle_request(client.clone(), "metrics", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "metrics response");
    
    let result = adapter.handle_request(client.clone(), "logs", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "logs response");
    
    let result = adapter.handle_request(client.clone(), "config", "{}", 1000).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "config response");
    
    let result = adapter.handle_request(client, "unknown", "{}", 1000).await;
    assert!(result.is_err());
}
