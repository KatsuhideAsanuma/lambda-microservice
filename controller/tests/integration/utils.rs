
use lambda_microservice_controller::{
    cache::RedisPool,
    config::Config,
    database::PostgresPool,
    session::SessionManager,
    runtime::RuntimeManager,
};
use actix_web::{test, App, web};
use std::sync::Arc;
use dotenv::dotenv;

pub async fn create_test_app() -> test::TestApp {
    dotenv().ok();

    let config = Config::from_env().expect("Failed to load configuration");

    let postgres_pool = PostgresPool::new(&config.database_url)
        .await
        .expect("Failed to create database connection pool");

    let redis_pool = RedisPool::new(&config.redis_url)
        .expect("Failed to create Redis connection pool");

    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));

    let runtime_manager = Arc::new(
        RuntimeManager::new(
            &config.runtime_config,
            postgres_pool.clone(),
        )
        .await
        .expect("Failed to initialize runtime manager"),
    );

    test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(runtime_manager.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(lambda_microservice_controller::api::configure)
    ).await
}

pub fn generate_test_id() -> String {
    use uuid::Uuid;
    format!("test-{}", Uuid::new_v4())
}

pub async fn cleanup_test_data(redis_pool: &RedisPool, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    redis_pool.del(key).await?;
    Ok(())
}
