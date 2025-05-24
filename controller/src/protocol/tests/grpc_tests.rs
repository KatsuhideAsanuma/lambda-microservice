use crate::error::{Error, Result};
use crate::protocol::grpc::{CircuitBreaker, CircuitBreakerConfig, GrpcProtocolAdapter, with_retry};
use std::time::Duration;
use serde_json::json;
use std::sync::Arc;

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

#[tokio::test]
async fn test_with_retry_success_after_failures() {
    let mock = MockGrpcAdapter::new(2);
    
    let result = with_retry("http://test:8080", "test", 3, || async {
        mock.test_operation().await
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(mock.failure_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_with_retry_all_failures() {
    let mock = MockGrpcAdapter::new(10); // More than max retries
    
    let result = with_retry("http://test:8080", "test", 3, || async {
        mock.test_operation().await
    }).await;
    
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            Error::Runtime(msg) => {
                assert!(msg.contains("Failed to execute") || msg.contains("after retries"));
            },
            _ => panic!("Expected Runtime error"),
        }
    }
    
    assert_eq!(mock.failure_count.load(std::sync::atomic::Ordering::SeqCst), 4);
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
    
    let result = adapter.handle_request(client, "unknown", "{}", 1000).await;
    assert!(result.is_err());
}
