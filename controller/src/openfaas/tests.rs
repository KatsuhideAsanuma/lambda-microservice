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
    
    #[test]
    fn test_client_configuration() {
        let client = OpenFaaSClient::new("http://test-gateway:8080", 45);
        
        assert_eq!(client.gateway_url, "http://test-gateway:8080");
        assert_eq!(client.timeout, std::time::Duration::from_secs(45));
    }
    
    #[test]
    fn test_parse_response() {
        let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
        
        let success_response = r#"{"result": {"output": "Hello, World!"}, "execution_time_ms": 123, "memory_usage_bytes": 1024}"#;
        let result = client.parse_response(success_response.as_bytes());
        assert!(result.is_ok());
        if let Ok(response) = result {
            assert_eq!(response.execution_time_ms, 123);
            assert_eq!(response.memory_usage_bytes, Some(1024));
            assert_eq!(response.result["output"], "Hello, World!");
        }
        
        let invalid_json = "not a json";
        let result = client.parse_response(invalid_json.as_bytes());
        assert!(result.is_err());
        
        let missing_fields = r#"{"execution_time_ms": 123}"#;
        let result = client.parse_response(missing_fields.as_bytes());
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod additional_tests {
    use super::super::*;
    use crate::error::{Error, Result};
    use crate::session::{Session, SessionStatus};
    use chrono::Utc;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_invoke_function_failure() {
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
        
        let client = OpenFaaSClient::new("http://invalid-host:8080", 1);
        let params = json!({"input": "test"});
        
        let result = client.invoke_function("nodejs-runtime", &session, params).await;
        
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                Error::Runtime(_) => {
                },
                _ => panic!("Expected Runtime error"),
            }
        }
    }
    
    #[test]
    fn test_timeout_handling() {
        let client = OpenFaaSClient::new("http://localhost:8080", 0); // 0秒タイムアウト
        
        assert_eq!(client.timeout, std::time::Duration::from_secs(0));
    }
    
    #[test]
    fn test_error_response_handling() {
        let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
        
        let error_response = r#"{"error": "Function execution failed", "execution_time_ms": 50}"#;
        let result = client.parse_response(error_response.as_bytes());
        
        assert!(result.is_ok());
        if let Ok(response) = result {
            assert_eq!(response.execution_time_ms, 50);
            assert_eq!(response.result["error"], "Function execution failed");
        }
    }
    
    #[test]
    fn test_large_payload_handling() {
        let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
        
        let large_script = "console.log('test');".repeat(1000); // 約17KBのスクリプト
        
        let session = Session {
            request_id: "test-request".to_string(),
            language_title: "nodejs".to_string(),
            user_id: Some("test-user".to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(1),
            last_executed_at: None,
            execution_count: 0,
            status: SessionStatus::Active,
            context: serde_json::json!({}),
            script_content: Some(large_script.clone()),
            script_hash: Some("test-hash".to_string()),
            compiled_artifact: None,
            compile_options: None,
            compile_status: Some("pending".to_string()),
            compile_error: None,
            metadata: None,
        };
        
        let params = json!({"input": "test"});
        
        let request = client.build_request("nodejs-runtime", &session, params);
        
        assert_eq!(request.script_content.unwrap(), large_script);
        
        let json_request = serde_json::to_string(&request).unwrap();
        
        assert!(json_request.len() > 17000); // 17KB以上あるはず
    }
}
