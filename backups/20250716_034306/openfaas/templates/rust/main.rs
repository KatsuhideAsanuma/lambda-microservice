use actix_cors::Cors;
use actix_web::{
    get, post,
    web::{self, Data, Json},
    App, HttpResponse, HttpServer,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;
use wasmtime::{Engine, Instance, Module, Store};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecuteRequest {
    request_id: String,
    params: serde_json::Value,
    context: serde_json::Value,
    script_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecuteResponse {
    result: serde_json::Value,
    execution_time_ms: u64,
    memory_usage_bytes: Option<u64>,
}

struct AppState {
    wasm_engine: Engine,
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

#[post("/execute")]
async fn execute(
    state: Data<Arc<AppState>>,
    request: Json<ExecuteRequest>,
) -> HttpResponse {
    let start_time = Instant::now();
    info!("Executing request {}", request.request_id);

    let execution_id = Uuid::new_v4().to_string();

    if request.script_content.is_none() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Script content is required for Rust runtime"
        }));
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let execution_time = start_time.elapsed().as_millis() as u64;
    info!("Request {} executed successfully in {}ms", request.request_id, execution_time);

    HttpResponse::Ok().json(ExecuteResponse {
        result: serde_json::json!({
            "result": "Simulated Rust WebAssembly execution result",
            "params": request.params,
        }),
        execution_time_ms: execution_time,
        memory_usage_bytes: Some(1024 * 1024), // 1MB
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let wasm_engine = Engine::default();
    let app_state = Arc::new(AppState {
        wasm_engine,
    });

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    info!("Starting Rust runtime on port {}", port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(Data::new(app_state.clone()))
            .service(health)
            .service(execute)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
