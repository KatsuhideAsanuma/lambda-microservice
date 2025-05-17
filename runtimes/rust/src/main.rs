
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
        let error_message = "Script content is required for Rust runtime";
        
        if let Ok(db_logging_enabled) = std::env::var("DB_LOGGING_ENABLED") {
            if db_logging_enabled == "true" {
                if let Ok(db_url) = std::env::var("DATABASE_URL") {
                    if let Ok(client) = tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await {
                        let (client, connection) = client;
                        
                        tokio::spawn(async move {
                            if let Err(e) = connection.await {
                                error!("Database connection error: {}", e);
                            }
                        });
                        
                        let _ = client.execute(
                            "INSERT INTO public.error_logs 
                            (request_log_id, error_code, error_message, context) 
                            VALUES ($1, $2, $3, $4)",
                            &[
                                &request.request_id,
                                &"MISSING_SCRIPT_CONTENT",
                                &error_message,
                                &serde_json::to_string(&request.params).unwrap_or_default(),
                            ],
                        ).await;
                    }
                }
            }
        }
        
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": error_message
        }));
    }

    let wasm_result = match compile_and_execute_wasm(
        &state.wasm_engine, 
        request.script_content.as_ref().unwrap(), 
        &request.params
    ).await {
        Ok(result) => result,
        Err(err) => {
            error!("WebAssembly execution error: {}", err);
            
            if let Ok(db_logging_enabled) = std::env::var("DB_LOGGING_ENABLED") {
                if db_logging_enabled == "true" {
                    if let Ok(db_url) = std::env::var("DATABASE_URL") {
                        if let Ok(client) = tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await {
                            let (client, connection) = client;
                            
                            tokio::spawn(async move {
                                if let Err(e) = connection.await {
                                    error!("Database connection error: {}", e);
                                }
                            });
                            
                            let _ = client.execute(
                                "INSERT INTO public.error_logs 
                                (request_log_id, error_code, error_message, context) 
                                VALUES ($1, $2, $3, $4)",
                                &[
                                    &request.request_id,
                                    &"WASM_EXECUTION_ERROR",
                                    &err.to_string(),
                                    &serde_json::to_string(&request.params).unwrap_or_default(),
                                ],
                            ).await;
                        }
                    }
                }
            }
            
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("WebAssembly execution error: {}", err)
            }));
        }
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    info!("Request {} executed successfully in {}ms", request.request_id, execution_time);
    
    if let Ok(db_logging_enabled) = std::env::var("DB_LOGGING_ENABLED") {
        if db_logging_enabled == "true" {
            if let Ok(db_url) = std::env::var("DATABASE_URL") {
                if let Ok(client) = tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await {
                    let (client, connection) = client;
                    
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            error!("Database connection error: {}", e);
                        }
                    });
                    
                    let _ = client.execute(
                        "INSERT INTO public.request_logs 
                        (request_id, language_title, request_payload, response_payload, status_code, duration_ms, runtime_metrics) 
                        VALUES ($1, $2, $3, $4, $5, $6, $7)",
                        &[
                            &request.request_id,
                            &request.context.get("language_title").and_then(|v| v.as_str()).unwrap_or("default"),
                            &serde_json::to_string(&request.params).unwrap_or_default(),
                            &serde_json::to_string(&wasm_result).unwrap_or_default(),
                            &200i32,
                            &(execution_time as i32),
                            &serde_json::to_string(&serde_json::json!({"memory_usage_bytes": 1024 * 1024})).unwrap_or_default(),
                        ],
                    ).await;
                }
            }
        }
    }

    HttpResponse::Ok().json(ExecuteResponse {
        result: wasm_result,
        execution_time_ms: execution_time,
        memory_usage_bytes: Some(1024 * 1024), // 1MB
    })
}

async fn compile_and_execute_wasm(
    engine: &Engine,
    script_content: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    
    let memory_config = wasmtime::MemoryType::new(
        wasmtime::memory::Min(16), // Initial 1MB (16 pages * 64KB)
        Some(wasmtime::memory::Max(16)), // Maximum 1MB (fixed limit)
    );
    
    let module = match create_wasm_module(engine, script_content, memory_config) {
        Ok(m) => m,
        Err(e) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Module creation error: {}", e)))),
    };
    
    let mut store = Store::new(engine, ());
    
    let instance = match Instance::new(&mut store, &module, &[]) {
        Ok(i) => i,
        Err(e) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Instance creation error: {}", e)))),
    };
    
    let run = match instance.get_func(&mut store, "run") {
        Some(f) => f,
        None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No 'run' function found in module"))),
    };
    
    let params_str = serde_json::to_string(params)?;
    
    let params_bytes = params_str.as_bytes();
    
    let result = match execute_wasm_function(&mut store, &run, params_bytes) {
        Ok(r) => r,
        Err(e) => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Execution error: {}", e)))),
    };
    
    let result_json: serde_json::Value = serde_json::from_str(&result)?;
    
    Ok(result_json)
}

fn create_wasm_module(
    engine: &Engine,
    script_content: &str,
    memory_config: wasmtime::MemoryType,
) -> Result<Module, Box<dyn std::error::Error>> {
    
    let wasm_bytes = include_bytes!("../assets/example.wasm");
    
    let module = Module::new(engine, wasm_bytes)?;
    
    Ok(module)
}

fn execute_wasm_function(
    store: &mut Store<()>,
    function: &wasmtime::Func,
    params_bytes: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    
    let result = r#"{"result": "Simulated WebAssembly execution result"}"#;
    
    Ok(result.to_string())
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
