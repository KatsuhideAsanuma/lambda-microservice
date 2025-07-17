use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Function error: {0}")]
    Function(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// From implementations for common error types
impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        Error::Database(err.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Error::Database(format!("Connection pool error: {}", err))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::BadRequest(format!("JSON serialization error: {}", err))
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::BadRequest(format!("UUID error: {}", err))
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Runtime(format!("HTTP client error: {}", err))
    }
}

// Actix-web integration
impl actix_web::ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::HttpResponse;

        match self {
            Error::Database(msg) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": msg
            })),
            Error::Session(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "error": "session_error",
                "message": msg
            })),
            Error::Function(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "error": "function_error",
                "message": msg
            })),
            Error::Runtime(msg) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "runtime_error",
                "message": msg
            })),
            Error::Config(msg) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "configuration_error",
                "message": msg
            })),
            Error::BadRequest(msg) => HttpResponse::BadRequest().json(serde_json::json!({
                "error": "validation_error",
                "message": msg
            })),
            Error::NotFound(msg) => HttpResponse::NotFound().json(serde_json::json!({
                "error": "not_found",
                "message": msg
            })),
            Error::Internal(msg) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "internal_error",
                "message": msg
            })),
        }
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;

        match self {
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Session(_) => StatusCode::BAD_REQUEST,
            Error::Function(_) => StatusCode::BAD_REQUEST,
            Error::Runtime(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
