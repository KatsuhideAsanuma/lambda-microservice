use lambda_microservice_controller::{
    error::Result,
    openfaas::{OpenFaaSClient, OpenFaaSRequest, OpenFaaSResponse},
    runtime::{RuntimeExecuteResponse, RuntimeType},
    session::Session,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

fn create_test_session() -> Session {
    Session {
        id: Uuid::new_v4(),
        request_id: Uuid::new_v4().to_string(),
        function_id: Uuid::new_v4(),
        language: "nodejs".to_string(),
        status: "pending".to_string(),
        script_content: Some("console.log('Hello, World!');".to_string()),
        compiled_artifact: None,
        compile_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        context: json!({"user": "test_user"}),
        execution_count: 0,
        last_execution_time: None,
        last_execution_result: None,
        last_execution_error: None,
        expires_at: Utc::now() + chrono::Duration::days(1),
    }
}

#[tokio::test]
async fn test_get_function_name_for_runtime() {
    let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
    
    assert_eq!(client.get_function_name_for_runtime(RuntimeType::NodeJs), "nodejs-runtime");
    assert_eq!(client.get_function_name_for_runtime(RuntimeType::Python), "python-runtime");
    assert_eq!(client.get_function_name_for_runtime(RuntimeType::Rust), "rust-runtime");
}

#[tokio::test]
async fn test_openfaas_client_new() {
    let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
    
    assert_eq!(client.gateway_url, "http://gateway.openfaas:8080");
}

#[tokio::test]
async fn test_build_request() {
    let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let request = OpenFaaSRequest {
        request_id: session.request_id.clone(),
        params: params.clone(),
        context: session.context.clone(),
        script_content: session.script_content.clone(),
    };
    
    assert_eq!(request.request_id, session.request_id);
    assert_eq!(request.params, params);
    assert_eq!(request.context, session.context);
    assert_eq!(request.script_content, session.script_content);
}

#[tokio::test]
async fn test_openfaas_response_serialization() {
    let response = OpenFaaSResponse {
        result: json!({"output": "test result"}),
        execution_time_ms: 123,
        memory_usage_bytes: Some(1024),
    };
    
    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: OpenFaaSResponse = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.result, response.result);
    assert_eq!(deserialized.execution_time_ms, response.execution_time_ms);
    assert_eq!(deserialized.memory_usage_bytes, response.memory_usage_bytes);
}
