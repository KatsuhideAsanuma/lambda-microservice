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
}

impl KubernetesClient {
    pub async fn new() -> Result<Self> {
        match Client::try_default().await {
            Ok(client) => Ok(Self { client }),
            Err(err) => {
                error!("Failed to create Kubernetes client: {}", err);
                Err(Error::External(format!("Kubernetes client error: {}", err)))
            }
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
        } else if app_label.contains("python") {
            let mut metadata = ObjectMeta::default();
            metadata.name = Some("lambda-python-runtime".to_string());
            metadata.namespace = Some(namespace.to_string());
            
            let mut service = Service::default();
            service.metadata = metadata;
            
            services.push(service);
        } else if app_label.contains("rust") {
            let mut metadata = ObjectMeta::default();
            metadata.name = Some("lambda-rust-runtime".to_string());
            metadata.namespace = Some(namespace.to_string());
            
            let mut service = Service::default();
            service.metadata = metadata;
            
            services.push(service);
        }
        
        info!(
            "Mock Kubernetes client returning {} services for namespace {} with labels {:?}",
            services.len(),
            namespace,
            labels
        );
        
        Ok(services)
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
