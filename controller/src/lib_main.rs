use actix_cors::Cors;
use actix_web::{middleware, web, App, Error, dev::{ServiceRequest, ServiceResponse}};
use crate::{
    api, config::Config, database::PostgresPool, function::FunctionManager, session::SessionManager,
    logger::DatabaseLogger, runtime::RuntimeManager, session::DbPoolTrait
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::FmtSubscriber;

pub fn init_tracing() -> FmtSubscriber {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    
    let _ = tracing::subscriber::set_global_default(subscriber);
    
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish()
}

pub async fn init_database(config: &Config) -> PostgresPool {
    let postgres_pool = PostgresPool::new(&config.database_url)
        .await
        .expect("Failed to create database connection pool");
    info!("Database connection pool initialized");
    postgres_pool
}

pub fn create_cors() -> Cors {
    Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600)
}

pub fn configure_app_for_testing() -> bool {
    true
}

pub fn configure_app_test<D, DL, RM>(
    _postgres_pool: D,
    _session_manager: Arc<SessionManager<D>>,
    _function_manager: Arc<FunctionManager<D>>,
    _db_logger: Arc<DL>,
    _runtime_manager: Arc<RM>,
    _config: Config,
) -> bool
where
    D: DbPoolTrait + Clone + 'static,
    DL: 'static,
    RM: 'static,
{
    true
}

pub fn configure_app<D, DL, RM>(
    postgres_pool: D,
    session_manager: Arc<SessionManager<D>>,
    function_manager: Arc<FunctionManager<D>>,
    db_logger: Arc<DL>,
    runtime_manager: Arc<RM>,
    config: Config,
) -> actix_web::Scope
where
    D: DbPoolTrait + Clone + 'static,
    DL: 'static,
    RM: 'static,
{
    let cors = create_cors();

    web::scope("")
        .app_data(web::Data::new(postgres_pool.clone()))
        .app_data(web::Data::new(session_manager.clone()))
        .app_data(web::Data::new(function_manager.clone()))
        .app_data(web::Data::new(db_logger.clone()))
        .app_data(web::Data::new(runtime_manager.clone()))
        .app_data(web::Data::new(config.clone()))
        .configure(api::configure)
}
