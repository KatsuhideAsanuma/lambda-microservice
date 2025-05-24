use lambda_microservice_controller::{
    error::{Error, Result},
    openfaas::{OpenFaaSClient, OpenFaaSRequest, OpenFaaSResponse},
    runtime::{RuntimeExecuteResponse, RuntimeType},
    session::{Session, SessionStatus},
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

fn create_test_session() -> Session {
    Session {
        request_id: Uuid::new_v4().to_string(),
        language_title: "nodejs-calculator".to_string(),
        user_id: Some("test-user".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(1),
        last_executed_at: None,
        execution_count: 0,
        status: SessionStatus::Active,
        context: json!({"user": "test_user"}),
        script_content: Some("console.log('Hello, World!');".to_string()),
        script_hash: Some("test-hash".to_string()),
        compiled_artifact: None,
        compile_options: None,
        compile_status: Some("pending".to_string()),
        compile_error: None,
        metadata: None,
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
    
    assert_eq!(client.get_function_name_for_runtime(RuntimeType::NodeJs), "nodejs-runtime");
}

#[tokio::test]
async fn test_build_request() {
    let client = OpenFaaSClient::new("http://gateway.openfaas:8080", 30);
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let request = client.build_request("nodejs-runtime", &session, params.clone());
    
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
    
    let runtime_response = RuntimeExecuteResponse {
        result: response.result.clone(),
        execution_time_ms: response.execution_time_ms,
        memory_usage_bytes: response.memory_usage_bytes,
    };
    
    assert_eq!(runtime_response.result, response.result);
    assert_eq!(runtime_response.execution_time_ms, response.execution_time_ms);
    assert_eq!(runtime_response.memory_usage_bytes, response.memory_usage_bytes);
}

#[tokio::test]
async fn test_openfaas_request_serialization() {
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let request = OpenFaaSRequest {
        request_id: session.request_id.clone(),
        params: params.clone(),
        context: session.context.clone(),
        script_content: session.script_content.clone(),
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    let deserialized: OpenFaaSRequest = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.request_id, request.request_id);
    assert_eq!(deserialized.params, request.params);
    assert_eq!(deserialized.context, request.context);
    assert_eq!(deserialized.script_content, request.script_content);
}

#[tokio::test]
async fn test_invoke_function_with_mock() {
    use lambda_microservice_controller::openfaas::tests::test_utils::MockOpenFaaSClient;
    
    let success_response = RuntimeExecuteResponse {
        result: json!({"output": "test result"}),
        execution_time_ms: 123,
        memory_usage_bytes: Some(1024),
    };
    
    let client = MockOpenFaaSClient::new()
        .with_invoke_result(Ok(success_response.clone()));
    
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let result = client.invoke_function("nodejs-runtime", &session, params).await;
    
    assert!(result.is_ok());
    if let Ok(response) = result {
        assert_eq!(response.execution_time_ms, success_response.execution_time_ms);
        assert_eq!(response.memory_usage_bytes, success_response.memory_usage_bytes);
        assert_eq!(response.result, success_response.result);
    }
}

#[tokio::test]
async fn test_invoke_function_error() {
    use lambda_microservice_controller::openfaas::tests::test_utils::MockOpenFaaSClient;
    
    let error = Error::Runtime("OpenFaaS function returned error status 500: Internal Server Error".to_string());
    
    let client = MockOpenFaaSClient::new()
        .with_invoke_result(Err(error.clone()));
    
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let result = client.invoke_function("nodejs-runtime", &session, params).await;
    
    assert!(result.is_err());
    if let Err(err) = result {
        match err {
            Error::Runtime(msg) => {
                assert!(msg.contains("error status 500"));
                assert!(msg.contains("Internal Server Error"));
            },
            _ => panic!("Expected Runtime error"),
        }
    }
}

#[tokio::test]
async fn test_invoke_function_parse_error() {
    use lambda_microservice_controller::openfaas::tests::test_utils::MockOpenFaaSClient;
    
    let error = Error::Runtime("Failed to parse OpenFaaS response: expected value at line 1 column 1".to_string());
    
    let client = MockOpenFaaSClient::new()
        .with_invoke_result(Err(error.clone()));
    
    let session = create_test_session();
    let params = json!({"input": "test"});
    
    let result = client.invoke_function("nodejs-runtime", &session, params).await;
    
    assert!(result.is_err());
    if let Err(err) = result {
        match err {
            Error::Runtime(msg) => {
                assert!(msg.contains("Failed to parse OpenFaaS response"));
            },
            _ => panic!("Expected Runtime error"),
        }
    }
}
