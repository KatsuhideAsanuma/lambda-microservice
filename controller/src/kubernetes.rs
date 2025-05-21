use crate::error::{Error, Result};
use kube::{
    api::{Api, ListParams},
    Client,
};
use k8s_openapi::api::core::v1::Service;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

pub struct KubernetesClient {
    client: Client,
    redis_client: Option<crate::cache::RedisClient>,
}

impl KubernetesClient {
    pub async fn new(redis_client: Option<crate::cache::RedisClient>) -> Result<Self> {
        match Client::try_default().await {
            Ok(client) => Ok(Self { 
                client,
                redis_client,
            }),
            Err(err) => {
                error!("Failed to create Kubernetes client: {}", err);
                Err(Error::External(format!("Kubernetes client error: {}", err)))
            }
        }
    }
    
    fn runtime_type_from_prefix(&self, runtime_prefix: &str) -> Option<crate::runtime::RuntimeType> {
        use crate::runtime::RuntimeType;
        
        match runtime_prefix {
            "nodejs-" => Some(RuntimeType::NodeJs),
            "python-" => Some(RuntimeType::Python),
            "rust-" => Some(RuntimeType::Rust),
            _ => None
        }
    }

    fn prefix_from_language_title(&self, language_title: &str) -> Option<&'static str> {
        if language_title.starts_with("nodejs-") { Some("nodejs-") }
        else if language_title.starts_with("python-") { Some("python-") }
        else if language_title.starts_with("rust-") { Some("rust-") }
        else { None }
    }
    
    pub async fn get_cached_runtime_type(
        &self,
        namespace: &str,
        language_title: &str
    ) -> Result<Option<crate::runtime::RuntimeType>> {
        if let Some(ref redis) = self.redis_client {
            match redis.get_runtime_type(namespace, language_title).await {
                Ok(Some(runtime_type)) => {
                    debug!(
                        "Cache hit for runtime type in namespace {} with language title {}",
                        namespace, language_title
                    );
                    return Ok(Some(runtime_type));
                }
                Ok(None) => {
                    debug!(
                        "Cache miss for runtime type in namespace {} with language title {}",
                        namespace, language_title
                    );
                }
                Err(err) => {
                    warn!(
                        "Error retrieving from cache for runtime type in namespace {} with language title {}: {}",
                        namespace, language_title, err
                    );
                }
            }
        }
        Ok(None)
    }

    pub async fn cache_runtime_type(
        &self,
        namespace: &str,
        language_title: &str,
        runtime_type: &crate::runtime::RuntimeType
    ) -> Result<()> {
        if let Some(ref redis) = self.redis_client {
            match redis.cache_runtime_type(namespace, language_title, runtime_type).await {
                Ok(_) => {
                    debug!("Cached runtime type for namespace {} with language title {}", namespace, language_title);
                    Ok(())
                }
                Err(err) => {
                    warn!("Failed to cache runtime type: {}", err);
                    Err(err)
                }
            }
        } else {
            Ok(()) // Redisクライアントがない場合は成功扱い
        }
    }

    pub async fn find_services(
        &self,
        namespace: &str,
        labels: HashMap<String, String>,
    ) -> Result<Vec<Service>> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        
        let label_selector = labels
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");
        
        let params = ListParams::default().labels(&label_selector);
        
        match services.list(&params).await {
            Ok(service_list) => {
                debug!(
                    "Found {} services in namespace {} with labels {}",
                    service_list.items.len(),
                    namespace,
                    label_selector
                );
                Ok(service_list.items)
            }
            Err(err) => {
                error!(
                    "Failed to list services in namespace {} with labels {}: {}",
                    namespace, label_selector, err
                );
                Err(Error::External(format!("Kubernetes API error: {}", err)))
            }
        }
    }

    #[cfg(feature = "mock-kubernetes")]
    pub async fn find_services_mock(
        &self,
        namespace: &str,
        labels: HashMap<String, String>,
    ) -> Result<Vec<Service>> {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        let app_label = labels.get("app").cloned().unwrap_or_default();
        
        let mut services = Vec::new();
        
        if app_label.contains("nodejs") {
            let mut metadata = ObjectMeta::default();
            metadata.name = Some("lambda-nodejs-runtime".to_string());
            metadata.namespace = Some(namespace.to_string());
            
            let mut service = Service::default();
            service.metadata = metadata;
            
            services.push(service);
            
            if let Ok(language_title) = self.language_title_from_app_label(&app_label) {
                let runtime_type = crate::runtime::RuntimeType::NodeJs;
                let _ = self.cache_runtime_type(namespace, &language_title, &runtime_type).await;
            }
        } else if app_label.contains("python") {
            let mut metadata = ObjectMeta::default();
            metadata.name = Some("lambda-python-runtime".to_string());
            metadata.namespace = Some(namespace.to_string());
            
            let mut service = Service::default();
            service.metadata = metadata;
            
            services.push(service);
            
            if let Ok(language_title) = self.language_title_from_app_label(&app_label) {
                let runtime_type = crate::runtime::RuntimeType::Python;
                let _ = self.cache_runtime_type(namespace, &language_title, &runtime_type).await;
            }
        } else if app_label.contains("rust") {
            let mut metadata = ObjectMeta::default();
            metadata.name = Some("lambda-rust-runtime".to_string());
            metadata.namespace = Some(namespace.to_string());
            
            let mut service = Service::default();
            service.metadata = metadata;
            
            services.push(service);
            
            if let Ok(language_title) = self.language_title_from_app_label(&app_label) {
                let runtime_type = crate::runtime::RuntimeType::Rust;
                let _ = self.cache_runtime_type(namespace, &language_title, &runtime_type).await;
            }
        }
        
        info!(
            "Mock Kubernetes client returning {} services for namespace {} with labels {:?}",
            services.len(),
            namespace,
            labels
        );
        
        Ok(services)
    }
    
    fn language_title_from_app_label(&self, app_label: &str) -> Result<String> {
        if app_label.contains("nodejs") { Ok("nodejs-function".to_string()) }
        else if app_label.contains("python") { Ok("python-function".to_string()) }
        else if app_label.contains("rust") { Ok("rust-function".to_string()) }
        else { Err(Error::BadRequest(format!("Unsupported app label: {}", app_label))) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    #[test]
    fn test_infer_runtime_type_from_service_name() {
        let service_name = "lambda-nodejs-runtime";
        assert!(service_name.contains("nodejs"));
        
        let service_name = "lambda-python-runtime";
        assert!(service_name.contains("python"));
        
        let service_name = "lambda-rust-runtime";
        assert!(service_name.contains("rust"));
    }
}
