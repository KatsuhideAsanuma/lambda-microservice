use lambda_microservice_controller::{
    error::Error,
    kubernetes::{KubernetesClientTrait, ServiceCache, KubernetesClient, KubernetesConfig, DeploymentConfig},
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

#[tokio::test]
async fn test_deployment_config_generation() {
    let config = KubernetesConfig {
        namespace: "default".to_string(),
        service_account: "lambda-controller".to_string(),
        deployment_replicas: 1,
        resource_limits_cpu: "500m".to_string(),
        resource_limits_memory: "512Mi".to_string(),
        resource_requests_cpu: "100m".to_string(),
        resource_requests_memory: "128Mi".to_string(),
        runtime_image: "lambda-runtime:latest".to_string(),
        registry_secret_name: Some("registry-creds".to_string()),
    };
    
    let client = KubernetesClient::new(config);
    
    let deployment_config = DeploymentConfig {
        name: "test-function".to_string(),
        image: "test-image:latest".to_string(),
        replicas: 2,
        env_vars: vec![
            ("NODE_ENV".to_string(), "production".to_string()),
            ("LOG_LEVEL".to_string(), "info".to_string()),
        ],
        labels: vec![
            ("app".to_string(), "test-function".to_string()),
            ("environment".to_string(), "test".to_string()),
        ],
    };
    
    let yaml = client.generate_deployment_yaml(&deployment_config);
    
    assert!(yaml.contains("kind: Deployment"));
    assert!(yaml.contains("name: test-function"));
    assert!(yaml.contains("replicas: 2"));
    assert!(yaml.contains("image: test-image:latest"));
    assert!(yaml.contains("name: NODE_ENV"));
    assert!(yaml.contains("value: production"));
    assert!(yaml.contains("app: test-function"));
    assert!(yaml.contains("environment: test"));
    
    assert!(yaml.contains("cpu: 500m"));
    assert!(yaml.contains("memory: 512Mi"));
    assert!(yaml.contains("cpu: 100m"));
    assert!(yaml.contains("memory: 128Mi"));
    
    assert!(yaml.contains("securityContext"));
    assert!(yaml.contains("runAsNonRoot: true"));
}

#[tokio::test]
async fn test_service_config_generation() {
    let config = KubernetesConfig {
        namespace: "default".to_string(),
        service_account: "lambda-controller".to_string(),
        deployment_replicas: 1,
        resource_limits_cpu: "500m".to_string(),
        resource_limits_memory: "512Mi".to_string(),
        resource_requests_cpu: "100m".to_string(),
        resource_requests_memory: "128Mi".to_string(),
        runtime_image: "lambda-runtime:latest".to_string(),
        registry_secret_name: Some("registry-creds".to_string()),
    };
    
    let client = KubernetesClient::new(config);
    
    let service_yaml = client.generate_service_yaml("test-service", 8080, vec![
        ("app".to_string(), "test-function".to_string()),
        ("tier".to_string(), "backend".to_string()),
    ]);
    
    assert!(service_yaml.contains("kind: Service"));
    assert!(service_yaml.contains("name: test-service"));
    assert!(service_yaml.contains("port: 8080"));
    assert!(service_yaml.contains("app: test-function"));
    assert!(service_yaml.contains("tier: backend"));
}
