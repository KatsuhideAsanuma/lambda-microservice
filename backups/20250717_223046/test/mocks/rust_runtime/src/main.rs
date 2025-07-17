use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use rand::Rng;

#[derive(Deserialize)]
struct RuntimeExecuteRequest {
    request_id: String,
    params: serde_json::Value,
    #[serde(default)]
    context: serde_json::Value,
    #[serde(default)]
    script_content: Option<String>,
}

#[derive(Serialize)]
struct RuntimeExecuteResponse {
    result: serde_json::Value,
    execution_time_ms: u64,
    memory_usage_bytes: Option<u64>,
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "runtime": "rust"
    }))
}

async fn execute(req: web::Json<RuntimeExecuteRequest>) -> impl Responder {
    let response_delay_ms: u64 = env::var("RESPONSE_DELAY_MS")
        .unwrap_or_else(|_| "50".to_string())
        .parse()
        .unwrap_or(50);
    
    let error_rate: f64 = env::var("ERROR_RATE")
        .unwrap_or_else(|_| "0.1".to_string())
        .parse()
        .unwrap_or(0.1);
    
    tokio::time::sleep(Duration::from_millis(response_delay_ms)).await;
    
    if rand::thread_rng().gen::<f64>() < error_rate {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Simulated runtime error"
        }));
    }
    
    let response = RuntimeExecuteResponse {
        result: serde_json::json!({
            "output": format!("Executed {} with params {}", req.request_id, req.params),
            "language": "rust"
        }),
        execution_time_ms: response_delay_ms,
        memory_usage_bytes: Some(1024 * 1024),
    };
    
    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8083".to_string())
        .parse()
        .unwrap_or(8083);
    
    println!("Mock Rust runtime server running on port {}", port);
    
    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health))
            .route("/execute", web::post().to(execute))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
