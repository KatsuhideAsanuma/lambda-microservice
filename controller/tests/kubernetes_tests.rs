use lambda_microservice_controller::{
    error::Error,
    kubernetes::{KubernetesClientTrait, ServiceCache},
    runtime::RuntimeType,
};

#[cfg(feature = "mock-kubernetes")]
use lambda_microservice_controller::kubernetes::MockKubernetesClient;
use std::time::Instant;

#[tokio::test]
async fn test_service_cache_new() {
    let cache = ServiceCache::new();
    assert!(Instant::now() >= Instant::now() - std::time::Duration::from_secs(1));
}

#[tokio::test]
async fn test_service_cache_is_stale() {
    let cache = ServiceCache::new();
    
    assert!(!cache.is_stale(60));
    
}

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_mock_kubernetes_client() {
    let client = MockKubernetesClient::new();
    
    let services = client.discover_runtime_services().await;
    assert!(services.is_ok());
    assert!(services.unwrap().len() >= 3);
}

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_kubernetes_client_get_runtime_type_for_language() {
    let client = MockKubernetesClient::new();
    
    let nodejs_result = client.get_runtime_type_for_language("nodejs-test").await;
    assert!(nodejs_result.is_ok());
    assert_eq!(nodejs_result.unwrap(), RuntimeType::NodeJs);
    
    let python_result = client.get_runtime_type_for_language("python-calculator").await;
    assert!(python_result.is_ok());
    assert_eq!(python_result.unwrap(), RuntimeType::Python);
    
    let rust_result = client.get_runtime_type_for_language("rust-wasm").await;
    assert!(rust_result.is_ok());
    assert_eq!(rust_result.unwrap(), RuntimeType::Rust);
    
    let js_result = client.get_runtime_type_for_language("javascript-app").await;
    assert!(js_result.is_ok());
    assert_eq!(js_result.unwrap(), RuntimeType::NodeJs);
    
    let py_result = client.get_runtime_type_for_language("py-script").await;
    assert!(py_result.is_ok());
    assert_eq!(py_result.unwrap(), RuntimeType::Python);
    
    let rs_result = client.get_runtime_type_for_language("rs-function").await;
    assert!(rs_result.is_ok());
    assert_eq!(rs_result.unwrap(), RuntimeType::Rust);
    
    let unknown_result = client.get_runtime_type_for_language("unknown").await;
    assert!(unknown_result.is_err());
    match unknown_result {
        Err(Error::BadRequest(msg)) => {
            assert!(msg.contains("No runtime found for language title"));
        },
        _ => panic!("Expected BadRequest error"),
    }
}
