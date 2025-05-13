
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] deadpool_postgres::PoolError),

    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] deadpool_redis::PoolError),

    #[error("Redis command error: {0}")]
    RedisCmd(#[from] redis::RedisError),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("WebAssembly error: {0}")]
    Wasm(String),

    #[error("Script compilation error: {0}")]
    Compilation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal server error: {0}")]
    InternalServer(String),
}

pub type Result<T> = std::result::Result<T, Error>;
