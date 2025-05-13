
use crate::error::{Error, Result};
use serde::Deserialize;
use std::env;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub session_expiry_seconds: u64,
    pub runtime_config: RuntimeConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeConfig {
    pub nodejs_runtime_url: String,
    pub python_runtime_url: String,
    pub rust_runtime_url: String,
    pub runtime_timeout_seconds: u64,
    pub max_script_size: usize,
    pub wasm_compile_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| Error::Config("Invalid PORT".to_string()))?,
            database_url: env::var("DATABASE_URL").map_err(|_| {
                Error::Config("DATABASE_URL environment variable not set".to_string())
            })?,
            redis_url: env::var("REDIS_URL").map_err(|_| {
                Error::Config("REDIS_URL environment variable not set".to_string())
            })?,
            session_expiry_seconds: env::var("SESSION_EXPIRY_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .map_err(|_| Error::Config("Invalid SESSION_EXPIRY_SECONDS".to_string()))?,
            runtime_config: RuntimeConfig {
                nodejs_runtime_url: env::var("NODEJS_RUNTIME_URL").map_err(|_| {
                    Error::Config("NODEJS_RUNTIME_URL environment variable not set".to_string())
                })?,
                python_runtime_url: env::var("PYTHON_RUNTIME_URL").map_err(|_| {
                    Error::Config("PYTHON_RUNTIME_URL environment variable not set".to_string())
                })?,
                rust_runtime_url: env::var("RUST_RUNTIME_URL").map_err(|_| {
                    Error::Config("RUST_RUNTIME_URL environment variable not set".to_string())
                })?,
                runtime_timeout_seconds: env::var("RUNTIME_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .map_err(|_| Error::Config("Invalid RUNTIME_TIMEOUT_SECONDS".to_string()))?,
                max_script_size: env::var("MAX_SCRIPT_SIZE")
                    .unwrap_or_else(|_| "1048576".to_string()) // 1MB
                    .parse()
                    .map_err(|_| Error::Config("Invalid MAX_SCRIPT_SIZE".to_string()))?,
                wasm_compile_timeout_seconds: env::var("WASM_COMPILE_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .map_err(|_| {
                        Error::Config("Invalid WASM_COMPILE_TIMEOUT_SECONDS".to_string())
                    })?,
            },
        })
    }
}
