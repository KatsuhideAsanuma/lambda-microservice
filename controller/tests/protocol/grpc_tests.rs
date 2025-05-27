use lambda_microservice_controller::{
    error::{Error, Result},
    protocol::grpc::{
        CircuitBreaker, CircuitBreakerConfig, GrpcProtocolAdapter, GrpcClient, RequestType,
        runtime::{
            runtime_service_client::RuntimeServiceClient,
            ExecuteRequest, ExecuteResponse,
            InitializeRequest, InitializeResponse,
            HealthCheckRequest, HealthCheckResponse,
            MetricsRequest, MetricsResponse,
            LogsRequest, LogsResponse,
            ConfigRequest, ConfigResponse,
        }
    },
};
use std::time::Duration;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_request_type_from_str() {
    assert_eq!(RequestType::from_str("execute"), Some(RequestType::Execute));
    assert_eq!(RequestType::from_str("initialize"), Some(RequestType::Initialize));
    assert_eq!(RequestType::from_str("health_check"), Some(RequestType::HealthCheck));
    assert_eq!(RequestType::from_str("metrics"), Some(RequestType::Metrics));
    assert_eq!(RequestType::from_str("logs"), Some(RequestType::Logs));
    assert_eq!(RequestType::from_str("config"), Some(RequestType::Config));
    assert_eq!(RequestType::from_str("unknown"), None);
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
    
    std::thread::sleep(Duration::from_millis(150));
    
    assert!(breaker.allow_request());
    
    breaker.record_success();
    assert!(breaker.allow_request());
    
    assert!(breaker.record_failure());
    assert!(!breaker.record_failure());
    assert!(!breaker.allow_request());
}

#[derive(Clone)]
struct MockGrpcClientWithHistory {
    request_history: Arc<TokioMutex<Vec<(String, String, u64)>>>,
    response: String,
    should_fail: bool,
}

impl MockGrpcClientWithHistory {
    fn new(response: String) -> Self {
        Self {
            request_history: Arc::new(TokioMutex::new(Vec::new())),
            response,
            should_fail: false,
        }
    }
    
    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    async fn get_request_history(&self) -> Vec<(String, String, u64)> {
        self.request_history.lock().await.clone()
    }
}

impl GrpcClient for MockGrpcClientWithHistory {
    async fn send_execute_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("execute".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated execute failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
    
    async fn send_initialize_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("initialize".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated initialize failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
    
    async fn send_health_check_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("health_check".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated health check failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
    
    async fn send_metrics_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("metrics".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated metrics failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
    
    async fn send_logs_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("logs".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated logs failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
    
    async fn send_config_request(&self, payload: String, timeout_ms: u64) -> Result<String> {
        self.request_history.lock().await.push(("config".to_string(), payload, timeout_ms));
        if self.should_fail {
            Err(Error::Runtime("Simulated config failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }
}

#[tokio::test]
async fn test_handle_request_success() {
    let adapter = GrpcProtocolAdapter::new();
    let client = Arc::new(MockGrpcClientWithHistory::new("success response".to_string()));
    
    let request_types = vec![
        "execute", "initialize", "health_check", "metrics", "logs", "config"
    ];
    
    for req_type in request_types {
        let result = adapter.handle_request(client.clone(), req_type, "{\"test\":\"data\"}", 1000).await;
        assert!(result.is_ok(), "Request type {} failed", req_type);
        assert_eq!(result.unwrap(), "success response");
    }
    
    let result = adapter.handle_request(client.clone(), "unknown", "{}", 1000).await;
    assert!(result.is_err());
    if let Err(Error::Runtime(msg)) = result {
        assert!(msg.contains("Unknown request type"));
    } else {
        panic!("Expected Runtime error");
    }
    
    let history = client.get_request_history().await;
    assert_eq!(history.len(), 6); // One for each valid request type
}

#[tokio::test]
async fn test_handle_request_failure() {
    let adapter = GrpcProtocolAdapter::new();
    let client = Arc::new(MockGrpcClientWithHistory::new("".to_string()).with_failure());
    
    let result = adapter.handle_request(client.clone(), "execute", "{}", 1000).await;
    assert!(result.is_err());
    if let Err(Error::Runtime(msg)) = result {
        assert!(msg.contains("Simulated execute failure"));
    } else {
        panic!("Expected Runtime error");
    }
}

#[tokio::test]
async fn test_degraded_operation() {
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
    failure_count: AtomicUsize,
    max_failures: usize,
}

impl MockGrpcAdapter {
    fn new(max_failures: usize) -> Self {
        Self {
            failure_count: AtomicUsize::new(0),
            max_failures,
        }
    }
    
    async fn test_operation(&self) -> Result<String> {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst);
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
    let adapter = GrpcProtocolAdapter::new();
    
    let result = adapter.with_retry("http://test:8080", "test", || async {
        mock.test_operation().await
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(mock.failure_count.load(Ordering::SeqCst), 3);
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
async fn test_send_request_execute() {
    let adapter = GrpcProtocolAdapter::new();
    
    let payload = json!({
        "request_type": "execute",
        "request_id": "test-123",
        "params": {"input": "test data"},
        "context": {"user": "test-user"},
        "script_content": "console.log('hello');"
    });
    
    let result = adapter.send_request(
        "http://localhost:50051", 
        payload.to_string().as_bytes(),
        1000
    ).await;
    
    assert!(result.is_ok());
    
    let response_json: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
    assert_eq!(response_json["result"], "Degraded operation: unable to execute normally");
    assert_eq!(response_json["degraded"], true);
}

#[tokio::test]
async fn test_send_request_health_check() {
    let adapter = GrpcProtocolAdapter::new();
    
    let payload = json!({
        "request_type": "health_check"
    });
    
    let result = adapter.send_request(
        "http://localhost:50051", 
        payload.to_string().as_bytes(),
        1000
    ).await;
    
    assert!(result.is_ok());
    
    let response_json: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
    assert_eq!(response_json["status"], "degraded");
    assert!(response_json["timestamp"].is_string());
}

#[tokio::test]
async fn test_send_request_invalid_json() {
    let adapter = GrpcProtocolAdapter::new();
    
    let payload = "not a valid json";
    
    let result = adapter.send_request(
        "http://localhost:50051", 
        payload.as_bytes(),
        1000
    ).await;
    
    assert!(result.is_err());
    if let Err(Error::Runtime(msg)) = result {
        assert!(msg.contains("Invalid JSON"));
    } else {
        panic!("Expected Runtime error about invalid JSON");
    }
}

#[tokio::test]
async fn test_send_request_invalid_utf8() {
    let adapter = GrpcProtocolAdapter::new();
    
    let payload = vec![0xFF, 0xFF, 0xFF];
    
    let result = adapter.send_request(
        "http://localhost:50051", 
        &payload,
        1000
    ).await;
    
    assert!(result.is_err());
    if let Err(Error::Runtime(msg)) = result {
        assert!(msg.contains("Invalid UTF-8"));
    } else {
        panic!("Expected Runtime error about invalid UTF-8");
    }
}

#[tokio::test]
async fn test_send_request_unknown_type() {
    let adapter = GrpcProtocolAdapter::new();
    
    let payload = json!({
        "request_type": "unknown_type",
        "request_id": "test-123"
    });
    
    let result = adapter.send_request(
        "http://localhost:50051", 
        payload.to_string().as_bytes(),
        1000
    ).await;
    
    assert!(result.is_err());
    if let Err(Error::Runtime(msg)) = result {
        assert!(msg.contains("Unknown request type"));
    } else {
        panic!("Expected Runtime error about unknown request type");
    }
}
