
use crate::error::Error;
use crate::runtime::RuntimeType;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[cfg(not(feature = "mock-kubernetes"))]
use kube::{
    api::{Api, ListParams},
    Client,
};

#[cfg(not(feature = "mock-kubernetes"))]
use k8s_openapi::api::core::v1::Service;

pub struct ServiceCache {
    pub services: HashMap<String, RuntimeType>,
    pub last_updated: Instant,
}

impl ServiceCache {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            last_updated: Instant::now(),
        }
    }

    pub fn is_stale(&self, ttl_seconds: u64) -> bool {
        let now = Instant::now();
        let age = now.duration_since(self.last_updated);
        age.as_secs() > ttl_seconds
    }
    
    pub fn get_services(&self) -> &HashMap<String, RuntimeType> {
        &self.services
    }
}

pub struct KubernetesClient {
    namespace: String,
    #[cfg(not(feature = "mock-kubernetes"))]
    client: Client,
    service_cache: Arc<RwLock<ServiceCache>>,
    cache_ttl_seconds: u64,
}

impl KubernetesClient {
    pub async fn new(namespace: &str, cache_ttl_seconds: u64) -> Result<Self, Error> {
        #[cfg(not(feature = "mock-kubernetes"))]
        let client = match Client::try_default().await {
            Ok(client) => {
                info!("Created Kubernetes client for namespace: {}", namespace);
                client
            },
            Err(e) => {
                error!("Failed to create Kubernetes client: {}", e);
                return Err(Error::External(format!("Failed to create Kubernetes client: {}", e)));
            }
        };

        #[cfg(feature = "mock-kubernetes")]
        info!("Creating mock Kubernetes client for namespace: {}", namespace);
        
        Ok(Self {
            namespace: namespace.to_string(),
            #[cfg(not(feature = "mock-kubernetes"))]
            client,
            service_cache: Arc::new(RwLock::new(ServiceCache::new())),
            cache_ttl_seconds,
        })
    }

    #[cfg(not(feature = "mock-kubernetes"))]
    pub async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>, Error> {
        {
            let cache = self.service_cache.read().await;
            if !cache.is_stale(self.cache_ttl_seconds) {
                debug!("Using cached runtime services");
                return Ok(cache.services.clone());
            }
        }

        info!("Discovering Kubernetes services in namespace: {}", self.namespace);
        
        let services_api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams::default()
            .labels("app.kubernetes.io/component=runtime");
        
        let services = match services_api.list(&lp).await {
            Ok(list) => list,
            Err(e) => {
                error!("Failed to list Kubernetes services: {}", e);
                return Err(Error::External(format!("Failed to list Kubernetes services: {}", e)));
            }
        };

        let mut runtime_services = HashMap::new();

        for service in services.items {
            let metadata = service.metadata;
                if let Some(labels) = metadata.labels {
                    if let Some(runtime_type) = labels.get("lambda.microservice/runtime") {
                        let service_name = metadata.name.unwrap_or_default();
                        
                        let runtime = match runtime_type.as_str() {
                            "nodejs" => RuntimeType::NodeJs,
                            "python" => RuntimeType::Python,
                            "rust" => RuntimeType::Rust,
                            _ => continue,
                        };
                        
                        runtime_services.insert(service_name.clone(), runtime);
                        info!("Discovered runtime service: {} of type {:?}", service_name, runtime);
                    }
                }
            }

        {
            let mut cache = self.service_cache.write().await;
            *cache = ServiceCache {
                services: runtime_services.clone(),
                last_updated: Instant::now(),
            };
        }

        Ok(runtime_services)
    }

    #[cfg(feature = "mock-kubernetes")]
    pub async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>, Error> {
        {
            let cache = self.service_cache.read().await;
            if !cache.is_stale(self.cache_ttl_seconds) {
                debug!("Using cached runtime services");
                return Ok(cache.services.clone());
            }
        }

        info!("Simulating Kubernetes service discovery in namespace: {}", self.namespace);
        
        let mut runtime_services = HashMap::new();
        
        runtime_services.insert("nodejs".to_string(), RuntimeType::NodeJs);
        runtime_services.insert("python".to_string(), RuntimeType::Python);
        runtime_services.insert("rust".to_string(), RuntimeType::Rust);
        
        runtime_services.insert("nodejs-api".to_string(), RuntimeType::NodeJs);
        runtime_services.insert("python-ml".to_string(), RuntimeType::Python);
        runtime_services.insert("rust-wasm".to_string(), RuntimeType::Rust);
        
        {
            let mut cache = self.service_cache.write().await;
            *cache = ServiceCache {
                services: runtime_services.clone(),
                last_updated: Instant::now(),
            };
        }

        Ok(runtime_services)
    }

    pub async fn get_runtime_type_for_language(&self, language_title: &str) -> Result<RuntimeType, Error> {
        info!("Getting runtime type for language: {}", language_title);
        
        let services = self.discover_runtime_services().await?;
        
        for (service_name, runtime_type) in &services {
            if language_title == service_name {
                info!("Found exact match for {}: {:?}", language_title, runtime_type);
                return Ok(*runtime_type);
            }
        }
        
        for (service_name, runtime_type) in &services {
            if language_title.starts_with(&format!("{}-", service_name)) {
                info!("Found prefix match for {}: {:?}", language_title, runtime_type);
                return Ok(*runtime_type);
            }
        }
        
        if language_title.contains("nodejs") || language_title.contains("node") || language_title.contains("javascript") || language_title.contains("js") {
            info!("Found keyword match for {}: NodeJs", language_title);
            return Ok(RuntimeType::NodeJs);
        } else if language_title.contains("python") || language_title.contains("py") {
            info!("Found keyword match for {}: Python", language_title);
            return Ok(RuntimeType::Python);
        } else if language_title.contains("rust") || language_title.contains("rs") {
            info!("Found keyword match for {}: Rust", language_title);
            return Ok(RuntimeType::Rust);
        }
        
        warn!("No runtime found for language title: {}", language_title);
        Err(Error::BadRequest(format!(
            "No runtime found for language title: {}",
            language_title
        )))
    }
}

#[async_trait]
pub trait KubernetesClientTrait: Send + Sync {
    async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>, Error>;
    
    async fn get_runtime_type_for_language(&self, language_title: &str) -> Result<RuntimeType, Error>;
}

#[async_trait]
impl KubernetesClientTrait for KubernetesClient {
    async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>, Error> {
        self.discover_runtime_services().await
    }
    
    async fn get_runtime_type_for_language(&self, language_title: &str) -> Result<RuntimeType, Error> {
        self.get_runtime_type_for_language(language_title).await
    }
}

#[cfg(feature = "mock-kubernetes")]
pub struct MockKubernetesClient {
    services: HashMap<String, RuntimeType>,
}

#[cfg(feature = "mock-kubernetes")]
impl MockKubernetesClient {
    pub fn new() -> Self {
        let mut services = HashMap::new();
        services.insert("nodejs".to_string(), RuntimeType::NodeJs);
        services.insert("python".to_string(), RuntimeType::Python);
        services.insert("rust".to_string(), RuntimeType::Rust);
        
        Self { services }
    }
}

#[cfg(feature = "mock-kubernetes")]
#[async_trait]
impl KubernetesClientTrait for MockKubernetesClient {
    async fn discover_runtime_services(&self) -> Result<HashMap<String, RuntimeType>, Error> {
        Ok(self.services.clone())
    }
    
    async fn get_runtime_type_for_language(&self, language_title: &str) -> Result<RuntimeType, Error> {
        for (service_name, runtime_type) in &self.services {
            if language_title == service_name {
                return Ok(*runtime_type);
            }
        }
        
        for (service_name, runtime_type) in &self.services {
            if language_title.starts_with(service_name) {
                return Ok(*runtime_type);
            }
        }
        
        if language_title.contains("nodejs") || language_title.contains("node") || language_title.contains("javascript") || language_title.contains("js") {
            return Ok(RuntimeType::NodeJs);
        } else if language_title.contains("python") || language_title.contains("py") {
            return Ok(RuntimeType::Python);
        } else if language_title.contains("rust") || language_title.contains("rs") {
            return Ok(RuntimeType::Rust);
        }
        
        Err(Error::BadRequest(format!(
            "No runtime found for language title: {}",
            language_title
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "mock-kubernetes")]
    #[tokio::test]
    async fn test_kubernetes_client() {
        let client = MockKubernetesClient::new();
        
        let services = client.discover_runtime_services().await.unwrap();
        assert!(services.len() >= 3);
        
        let runtime_type = client.get_runtime_type_for_language("nodejs-test").await.unwrap();
        assert_eq!(runtime_type, RuntimeType::NodeJs);
        
        let runtime_type = client.get_runtime_type_for_language("python-calculator").await.unwrap();
        assert_eq!(runtime_type, RuntimeType::Python);
        
        let runtime_type = client.get_runtime_type_for_language("rust-wasm").await.unwrap();
        assert_eq!(runtime_type, RuntimeType::Rust);
        
        let result = client.get_runtime_type_for_language("unknown").await;
        assert!(result.is_err());
    }
}
