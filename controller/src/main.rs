use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use lambda_microservice_controller::{
    api, config::Config, database::PostgresPool, function::FunctionManager, session::SessionManager,
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let config = Config::from_env().expect("Failed to load configuration");
    info!("Configuration loaded");

    let postgres_pool = PostgresPool::new(&config.database_url)
        .await
        .expect("Failed to create database connection pool");
    info!("Database connection pool initialized");

    let redis_pool = lambda_microservice_controller::cache::RedisPool::new(&config.redis_url)
        .expect("Failed to create Redis connection pool");
    info!("Redis connection pool initialized");

    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));
    info!("Session manager initialized");

    let function_manager = Arc::new(
        FunctionManager::new(postgres_pool.clone())
    );
    info!("Function manager initialized");

    let db_logger = Arc::new(
        lambda_microservice_controller::logger::DatabaseLogger::new(postgres_pool.clone().into(), true)
    );
    info!("Database logger initialized");

    let runtime_manager = Arc::new(
        lambda_microservice_controller::runtime::RuntimeManager::new(
            &config.runtime_config,
            postgres_pool.clone(),
        )
        .await
        .expect("Failed to initialize runtime manager"),
    );
    info!("Runtime manager initialized");

    info!("Starting server at {}:{}", config.host, config.port);
    
    let bind_config = config.clone();
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(TracingLogger::default())
            .wrap(middleware::Compress::default())
            .wrap(cors)
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(function_manager.clone()))
            .app_data(web::Data::new(db_logger.clone()))
            .app_data(web::Data::new(runtime_manager.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(api::configure)
    })
    .bind(format!("{}:{}", bind_config.host, bind_config.port))?
    .run()
    .await
}
