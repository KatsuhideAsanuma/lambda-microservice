use lambda_microservice_controller::{
    error::Error,
    kubernetes::{KubernetesClientTrait, ServiceCache, KubernetesClient},
    runtime::RuntimeType,
};

#[cfg(feature = "mock-kubernetes")]
use lambda_microservice_controller::kubernetes::MockKubernetesClient;
use std::time::{Instant, Duration};
use std::collections::HashMap;

#[tokio::test]
async fn test_service_cache_new() {
    let cache = ServiceCache::new();
    assert!(Instant::now() >= cache.last_updated - Duration::from_secs(1));
    assert_eq!(cache.services.len(), 0);
}

#[tokio::test]
async fn test_service_cache_is_stale() {
    let cache = ServiceCache::new();
    
    assert!(!cache.is_stale(60));
    
    assert!(!cache.is_stale(0));
    
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert!(cache.is_stale(0));
}

#[tokio::test]
async fn test_service_cache_get_services() {
    let mut cache = ServiceCache::new();
    
    let services = cache.get_services();
    assert_eq!(services.len(), 0);
    
    let mut services_map = HashMap::new();
    services_map.insert("test-service".to_string(), RuntimeType::NodeJs);
    cache.services = services_map;
    
    let updated_services = cache.get_services();
    assert_eq!(updated_services.len(), 1);
    assert_eq!(updated_services.get("test-service"), Some(&RuntimeType::NodeJs));
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
    
    let nodejs_result = client.get_runtime_type_for_language("nodejs").await;
    assert!(nodejs_result.is_ok());
    assert_eq!(nodejs_result.unwrap(), RuntimeType::NodeJs);
    
    let nodejs_test_result = client.get_runtime_type_for_language("nodejs-test").await;
    assert!(nodejs_test_result.is_ok());
    assert_eq!(nodejs_test_result.unwrap(), RuntimeType::NodeJs);
    
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

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_kubernetes_client_creation() {
    let client = KubernetesClient::new("default", 60).await;
    assert!(client.is_ok(), "KubernetesClient creation should succeed");
    
    let client = client.unwrap();
    
    let mock_client = MockKubernetesClient::new();
    
    let services = mock_client.discover_runtime_services().await;
    assert!(services.is_ok(), "discover_runtime_services should succeed");
    
    let services = services.unwrap();
    assert!(!services.is_empty(), "Expected non-empty services map");
    
    assert!(services.contains_key("nodejs"), "Expected 'nodejs' service");
    assert!(services.contains_key("python"), "Expected 'python' service");
    assert!(services.contains_key("rust"), "Expected 'rust' service");
    
    assert_eq!(services.get("nodejs"), Some(&RuntimeType::NodeJs));
    assert_eq!(services.get("python"), Some(&RuntimeType::Python));
    assert_eq!(services.get("rust"), Some(&RuntimeType::Rust));
}

#[tokio::test]
async fn test_service_cache_update() {
    let cache = ServiceCache::new();
    
    assert!(!cache.is_stale(60));
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    assert!(cache.is_stale(0));
    
    let services = cache.get_services();
    assert_eq!(services.len(), 0);
    
    let new_cache = ServiceCache::new();
    assert!(!new_cache.is_stale(60));
    assert_eq!(new_cache.get_services().len(), 0);
}

#[tokio::test]
async fn test_service_cache_methods() {
    let cache = ServiceCache::new();
    
    assert!(!cache.is_stale(60));
    assert_eq!(cache.get_services().len(), 0);
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    assert!(cache.is_stale(0), "Cache should be stale with TTL=0 after 1 second");
    assert!(!cache.is_stale(10), "Cache should not be stale with TTL=10 after 1 second");
    
    let fresh_cache = ServiceCache::new();
    assert!(!fresh_cache.is_stale(60));
    assert_eq!(fresh_cache.get_services().len(), 0);
}

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_mock_kubernetes_client_methods() {
    let client = MockKubernetesClient::new();
    
    let services = client.discover_runtime_services().await.unwrap();
    assert!(services.len() >= 3);
    assert!(services.contains_key("nodejs"));
    assert!(services.contains_key("python"));
    assert!(services.contains_key("rust"));
    
    assert_eq!(
        client.get_runtime_type_for_language("nodejs").await.unwrap(),
        RuntimeType::NodeJs
    );
    
    assert_eq!(
        client.get_runtime_type_for_language("nodejs-api").await.unwrap(),
        RuntimeType::NodeJs
    );
    
    assert_eq!(
        client.get_runtime_type_for_language("javascript").await.unwrap(),
        RuntimeType::NodeJs
    );
}

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_kubernetes_client_discover_runtime_services() {
    let client = MockKubernetesClient::new();
    
    let services1 = client.discover_runtime_services().await.unwrap();
    assert!(!services1.is_empty());
    
    let services2 = client.discover_runtime_services().await.unwrap();
    assert_eq!(services1.len(), services2.len());
}

#[cfg(feature = "mock-kubernetes")]
#[tokio::test]
async fn test_kubernetes_client_trait_implementation() {
    let mock_client = MockKubernetesClient::new();
    
    let services = KubernetesClientTrait::discover_runtime_services(&mock_client).await.unwrap();
    assert!(!services.is_empty());
    
    let runtime_type = KubernetesClientTrait::get_runtime_type_for_language(&mock_client, "nodejs").await.unwrap();
    assert_eq!(runtime_type, RuntimeType::NodeJs);
    
    let runtime_type = KubernetesClientTrait::get_runtime_type_for_language(&mock_client, "nodejs-api").await.unwrap();
    assert_eq!(runtime_type, RuntimeType::NodeJs);
    
    let runtime_type = KubernetesClientTrait::get_runtime_type_for_language(&mock_client, "python").await.unwrap();
    assert_eq!(runtime_type, RuntimeType::Python);
}
