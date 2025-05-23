#[cfg(feature = "test-isolated")]
mod kubernetes_tests {
    use crate::error::Result;
    use crate::kubernetes::{KubernetesClient, KubernetesClientTrait};
    use crate::runtime::RuntimeType;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::RwLock;
    use async_trait::async_trait;

    pub struct MockKubernetesClient {
        services: HashMap<String, RuntimeType>,
    }

    impl MockKubernetesClient {
        pub fn new() -> Self {
            let mut services = HashMap::new();
            services.insert("nodejs".to_string(), RuntimeType::NodeJs);
            services.insert("python".to_string(), RuntimeType::Python);
            services.insert("rust".to_string(), RuntimeType::Rust);
            Self { services }
        }
    }

    #[async_trait]
    impl KubernetesClientTrait for MockKubernetesClient {
        async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>> {
            Ok(self.services.clone())
        }
        
        async fn get_runtime_type_for_language(&self, language_title: &str) -> Result<RuntimeType> {
            if language_title.starts_with("nodejs") {
                return Ok(RuntimeType::NodeJs);
            } else if language_title.starts_with("python") {
                return Ok(RuntimeType::Python);
            } else if language_title.starts_with("rust") {
                return Ok(RuntimeType::Rust);
            }
            
            if language_title.contains("javascript") || language_title.contains("node") {
                return Ok(RuntimeType::NodeJs);
            } else if language_title.contains("py") {
                return Ok(RuntimeType::Python);
            } else if language_title.contains("rs") {
                return Ok(RuntimeType::Rust);
            }
            
            Err(crate::error::Error::BadRequest(format!(
                "No matching runtime found for language title: {}",
                language_title
            )))
        }
    }

    #[tokio::test]
    async fn test_mock_kubernetes_client() {
        let client = MockKubernetesClient::new();
        
        let services = client.discover_runtime_services().await.unwrap();
        assert_eq!(services.len(), 3);
        
        assert_eq!(client.get_runtime_type_for_language("nodejs-test").await.unwrap(), RuntimeType::NodeJs);
        assert_eq!(client.get_runtime_type_for_language("python-calculator").await.unwrap(), RuntimeType::Python);
        assert_eq!(client.get_runtime_type_for_language("rust-wasm").await.unwrap(), RuntimeType::Rust);
        
        assert_eq!(client.get_runtime_type_for_language("test-with-javascript").await.unwrap(), RuntimeType::NodeJs);
        
        assert!(client.get_runtime_type_for_language("unknown-language").await.is_err());
    }
}
