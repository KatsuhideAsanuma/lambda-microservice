
use crate::integration::utils::{create_test_app, generate_test_id, cleanup_test_data};
use actix_web::{test, http::header, http::StatusCode};
use lambda_microservice_controller::cache::RedisPool;
use serde_json::{json, Value};

#[actix_rt::test]
#[ignore]
async fn test_initialize_endpoint() {
    let app = create_test_app().await;
    
    let test_id = generate_test_id();
    
    let req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "nodejs-calculator"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "module.exports = async (event) => { const { values } = event.params; return { result: values.reduce((a, b) => a + b, 0) }; }"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::OK);
    
    let body: Value = test::read_body_json(resp).await;
    
    assert!(body.get("request_id").is_some());
    assert!(body.get("expiry").is_some());
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let request_id = body["request_id"].as_str().unwrap();
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}

#[actix_rt::test]
#[ignore]
async fn test_execute_endpoint() {
    let app = create_test_app().await;
    
    let test_id = generate_test_id();
    
    let init_req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "nodejs-calculator"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "module.exports = async (event) => { const { values } = event.params; return { result: values.reduce((a, b) => a + b, 0) }; }"
        }))
        .to_request();
    
    let init_resp = test::call_service(&app, init_req).await;
    assert_eq!(init_resp.status(), StatusCode::OK);
    
    let init_body: Value = test::read_body_json(init_resp).await;
    let request_id = init_body["request_id"].as_str().unwrap();
    
    let exec_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let exec_resp = test::call_service(&app, exec_req).await;
    assert_eq!(exec_resp.status(), StatusCode::OK);
    
    let exec_body: Value = test::read_body_json(exec_resp).await;
    
    assert!(exec_body.get("result").is_some());
    assert_eq!(exec_body["result"]["value"], 15);
    assert_eq!(exec_body["cached"], false);
    
    let exec_req2 = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let exec_resp2 = test::call_service(&app, exec_req2).await;
    assert_eq!(exec_resp2.status(), StatusCode::OK);
    
    let exec_body2: Value = test::read_body_json(exec_resp2).await;
    
    assert!(exec_body2.get("result").is_some());
    assert_eq!(exec_body2["result"]["value"], 15);
    assert_eq!(exec_body2["cached"], true);
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}

#[actix_rt::test]
#[ignore]
async fn test_invalid_language_title() {
    let app = create_test_app().await;
    
    let req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "invalid-language"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "console.log('Hello');"
        }))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    
    let body: Value = test::read_body_json(resp).await;
    
    assert!(body.get("error").is_some());
    assert!(body["error"].as_str().unwrap().contains("Invalid Language-Title"));
}
