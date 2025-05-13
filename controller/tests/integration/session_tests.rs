
use crate::integration::utils::{create_test_app, generate_test_id, cleanup_test_data};
use actix_web::{test, http::header, http::StatusCode};
use lambda_microservice_controller::cache::RedisPool;
use serde_json::{json, Value};
use std::time::Duration;

#[actix_rt::test]
#[ignore]
async fn test_session_persistence() {
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
    
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let info_req = test::TestRequest::get()
        .uri(&format!("/api/v1/session/{}/info", request_id))
        .to_request();
    
    let info_resp = test::call_service(&app, info_req).await;
    assert_eq!(info_resp.status(), StatusCode::OK);
    
    let info_body: Value = test::read_body_json(info_resp).await;
    
    assert_eq!(info_body["language_title"], "nodejs-calculator");
    assert_eq!(info_body["execution_count"], 1);
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}

#[actix_rt::test]
#[ignore]
async fn test_session_expiry() {
    
    let app = create_test_app().await;
    
    let expired_id = "expired-session-id";
    
    let exec_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", expired_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "values": [1, 2, 3]
            }
        }))
        .to_request();
    
    let exec_resp = test::call_service(&app, exec_req).await;
    assert_eq!(exec_resp.status(), StatusCode::NOT_FOUND);
    
    let info_req = test::TestRequest::get()
        .uri(&format!("/api/v1/session/{}/info", expired_id))
        .to_request();
    
    let info_resp = test::call_service(&app, info_req).await;
    assert_eq!(info_resp.status(), StatusCode::NOT_FOUND);
}
