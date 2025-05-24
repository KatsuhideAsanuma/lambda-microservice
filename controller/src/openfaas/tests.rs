#[cfg(feature = "test-integration")]
pub mod test_utils {
    use super::super::*;
    use crate::runtime::RuntimeExecuteResponse;
    use crate::session::Session;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    #[derive(Clone)]
    pub struct MockOpenFaaSClient {
        invoke_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
    }
    
    impl MockOpenFaaSClient {
        pub fn new() -> Self {
            Self {
                invoke_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                    result: serde_json::json!({"status": "success"}),
                    execution_time_ms: 100,
                    memory_usage_bytes: Some(1024),
                }))),
            }
        }
        
        pub fn with_invoke_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
            self.invoke_result = Arc::new(Mutex::new(result));
            self
        }
        
        pub async fn invoke_function(
            &self,
            _function_name: &str,
            _session: &Session,
            _params: serde_json::Value,
        ) -> Result<RuntimeExecuteResponse> {
            self.invoke_result.lock().await.clone()
        }
        
        pub fn get_function_name_for_runtime(&self, runtime_type: RuntimeType) -> String {
            match runtime_type {
                RuntimeType::NodeJs => "nodejs-runtime".to_string(),
                RuntimeType::Python => "python-runtime".to_string(),
                RuntimeType::Rust => "rust-runtime".to_string(),
            }
        }
        
        pub fn build_request(&self, _function_name: &str, session: &Session, params: serde_json::Value) -> OpenFaaSRequest {
            OpenFaaSRequest {
                request_id: session.request_id.clone(),
                params,
                context: session.context.clone(),
                script_content: session.script_content.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::runtime::RuntimeType;
    
    #[test]
    fn test_get_function_name_for_runtime() {
        let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
        
        assert_eq!(client.get_function_name_for_runtime(RuntimeType::NodeJs), "nodejs-runtime");
        assert_eq!(client.get_function_name_for_runtime(RuntimeType::Python), "python-runtime");
        assert_eq!(client.get_function_name_for_runtime(RuntimeType::Rust), "rust-runtime");
    }
    
    #[test]
    fn test_build_request() {
        use crate::session::{Session, SessionStatus};
        use chrono::Utc;
        
        let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
        
        let session = Session {
            request_id: "test-request".to_string(),
            language_title: "nodejs".to_string(),
            user_id: Some("test-user".to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(1),
            last_executed_at: None,
            execution_count: 0,
            status: SessionStatus::Active,
            context: serde_json::json!({"test": "data"}),
            script_content: Some("console.log('test');".to_string()),
            script_hash: Some("test-hash".to_string()),
            compiled_artifact: None,
            compile_options: None,
            compile_status: Some("pending".to_string()),
            compile_error: None,
            metadata: None,
        };
        
        let params = serde_json::json!({"input": "test"});
        
        let request = client.build_request("nodejs-runtime", &session, params.clone());
        
        assert_eq!(request.request_id, session.request_id);
        assert_eq!(request.params, params);
        assert_eq!(request.context, session.context);
        assert_eq!(request.script_content, session.script_content);
    }
}
