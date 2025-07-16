
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("PostgreSQL error: {0}")]
    Postgres(String),

    #[error("Redis error: {0}")]
    Cache(String),

    #[error("Redis command error: {0}")]
    RedisCmd(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("HTTP client error: {0}")]
    External(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

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

    #[error("IO error: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Error::Database(err.to_string())
    }
}


impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::External(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error::Postgres(err.to_string())
    }
}


impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Database(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let db_error = Error::Database("Connection failed".to_string());
        assert!(db_error.to_string().contains("Connection failed"));

        let cache_error = Error::Cache("Redis error".to_string());
        assert!(cache_error.to_string().contains("Redis error"));

        let runtime_error = Error::Runtime("Execution failed".to_string());
        assert!(runtime_error.to_string().contains("Execution failed"));

        let bad_request = Error::BadRequest("Invalid parameters".to_string());
        assert!(bad_request.to_string().contains("Invalid parameters"));

        let not_found = Error::NotFound("Session not found".to_string());
        assert!(not_found.to_string().contains("Session not found"));

        let compilation = Error::Compilation("Failed to compile".to_string());
        assert!(compilation.to_string().contains("Failed to compile"));

        let io_error = Error::Io("IO error".to_string());
        assert!(io_error.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_from_reqwest() {
        let reqwest_error = reqwest::Client::new()
            .get("invalid-url")
            .build()
            .unwrap_err();
        let error = Error::from(reqwest_error);
        assert!(matches!(error, Error::External(_)));
    }


    #[test]
    fn test_error_from_sqlx() {
        let sqlx_error = sqlx::Error::RowNotFound;
        let error = Error::from(sqlx_error);
        assert!(matches!(error, Error::Database(_)));
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error = Error::from(json_error);
        assert!(matches!(error, Error::Serialization(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = Error::from(io_error);
        assert!(matches!(error, Error::Io(_)));
    }
}
