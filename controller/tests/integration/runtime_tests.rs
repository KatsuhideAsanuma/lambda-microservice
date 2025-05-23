
use crate::integration::utils::{create_test_app, generate_test_id, cleanup_test_data};
use actix_web::{test, http::header, http::StatusCode};
use lambda_microservice_controller::cache::RedisPool;
use serde_json::{json, Value};

#[actix_rt::test]
async fn test_nodejs_runtime() {
    let app = create_test_app().await;
    
    let init_req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "nodejs-calculator"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "module.exports = async (event) => { 
                const { operation, values } = event.params; 
                let result;
                switch(operation) {
                    case 'add':
                        result = values.reduce((a, b) => a + b, 0);
                        break;
                    case 'multiply':
                        result = values.reduce((a, b) => a * b, 1);
                        break;
                    default:
                        throw new Error('Unsupported operation');
                }
                return { result };
            }"
        }))
        .to_request();
    
    let init_resp = test::call_service(&app, init_req).await;
    assert_eq!(init_resp.status(), StatusCode::OK);
    
    let init_body: Value = test::read_body_json(init_resp).await;
    let request_id = init_body["request_id"].as_str().unwrap();
    
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "operation": "add",
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::OK);
    
    let add_body: Value = test::read_body_json(add_resp).await;
    assert_eq!(add_body["result"]["result"], 15);
    
    let mul_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "operation": "multiply",
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let mul_resp = test::call_service(&app, mul_req).await;
    assert_eq!(mul_resp.status(), StatusCode::OK);
    
    let mul_body: Value = test::read_body_json(mul_resp).await;
    assert_eq!(mul_body["result"]["result"], 120);
    
    let err_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "operation": "unsupported",
                "values": [1, 2, 3]
            }
        }))
        .to_request();
    
    let err_resp = test::call_service(&app, err_req).await;
    assert_eq!(err_resp.status(), StatusCode::BAD_REQUEST);
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}

#[actix_rt::test]
async fn test_python_runtime() {
    let app = create_test_app().await;
    
    let init_req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "python-calculator"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "
def handle(event):
    params = event.get('params', {})
    operation = params.get('operation')
    values = params.get('values', [])
    
    if operation == 'add':
        result = sum(values)
    elif operation == 'multiply':
        result = 1
        for val in values:
            result *= val
    else:
        raise ValueError('Unsupported operation')
        
    return {'result': result}
"
        }))
        .to_request();
    
    let init_resp = test::call_service(&app, init_req).await;
    assert_eq!(init_resp.status(), StatusCode::OK);
    
    let init_body: Value = test::read_body_json(init_resp).await;
    let request_id = init_body["request_id"].as_str().unwrap();
    
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "operation": "add",
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::OK);
    
    let add_body: Value = test::read_body_json(add_resp).await;
    assert_eq!(add_body["result"]["result"], 15);
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}

#[actix_rt::test]
async fn test_rust_runtime() {
    let app = create_test_app().await;
    
    let init_req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(header::ContentType::json())
        .insert_header(("Language-Title", "rust-calculator"))
        .set_json(json!({
            "context": {
                "environment": "test"
            },
            "script_content": "
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
struct Params {
    operation: String,
    values: Vec<i64>,
}

#[derive(Serialize)]
struct Response {
    result: i64,
}

pub fn handle(event: &str) -> Result<String, String> {
    let event: Value = serde_json::from_str(event).map_err(|e| e.to_string())?;
    let params: Params = serde_json::from_value(event[\"params\"].clone()).map_err(|e| e.to_string())?;
    
    let result = match params.operation.as_str() {
        \"add\" => params.values.iter().sum(),
        \"multiply\" => params.values.iter().fold(1, |acc, &x| acc * x),
        _ => return Err(\"Unsupported operation\".to_string()),
    };
    
    let response = Response { result };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}
"
        }))
        .to_request();
    
    let init_resp = test::call_service(&app, init_req).await;
    assert_eq!(init_resp.status(), StatusCode::OK);
    
    let init_body: Value = test::read_body_json(init_resp).await;
    let request_id = init_body["request_id"].as_str().unwrap();
    
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .insert_header(header::ContentType::json())
        .set_json(json!({
            "params": {
                "operation": "add",
                "values": [1, 2, 3, 4, 5]
            }
        }))
        .to_request();
    
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::OK);
    
    let add_body: Value = test::read_body_json(add_resp).await;
    assert_eq!(add_body["result"]["result"], 15);
    
    let redis_pool = app
        .app_data::<web::Data<RedisPool>>()
        .expect("Failed to get Redis pool");
    
    let _ = cleanup_test_data(redis_pool.get_ref(), &format!("session:{}", request_id)).await;
}
