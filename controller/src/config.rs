
use crate::error::{Error, Result};
use serde::Deserialize;
use std::env;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub session_expiry_seconds: u64,
    pub runtime_config: RuntimeConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    pub nodejs_runtime_url: String,
    pub python_runtime_url: String,
    pub rust_runtime_url: String,
    pub runtime_timeout_seconds: u64,
    pub runtime_fallback_timeout_seconds: u64,
    pub runtime_max_retries: u32,
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
                runtime_fallback_timeout_seconds: env::var("RUNTIME_FALLBACK_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()
                    .map_err(|_| Error::Config("Invalid RUNTIME_FALLBACK_TIMEOUT_SECONDS".to_string()))?,
                runtime_max_retries: env::var("RUNTIME_MAX_RETRIES")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .map_err(|_| Error::Config("Invalid RUNTIME_MAX_RETRIES".to_string()))?,
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

    #[cfg(test)]
    pub fn from_values(
        host: &str,
        port: u16,
        database_url: &str,
        redis_url: &str,
        session_expiry_seconds: u64,
        runtime_config: RuntimeConfig,
    ) -> Self {
        Self {
            host: host.to_string(),
            port,
            database_url: database_url.to_string(),
            redis_url: redis_url.to_string(),
            session_expiry_seconds,
            runtime_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_env_vars() {
        env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/testdb");
        env::set_var("REDIS_URL", "redis://localhost:6379");
        env::set_var("NODEJS_RUNTIME_URL", "http://localhost:8081");
        env::set_var("PYTHON_RUNTIME_URL", "http://localhost:8082");
        env::set_var("RUST_RUNTIME_URL", "http://localhost:8083");
    }

    fn clear_env_vars() {
        env::remove_var("HOST");
        env::remove_var("PORT");
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");
        env::remove_var("SESSION_EXPIRY_SECONDS");
        env::remove_var("NODEJS_RUNTIME_URL");
        env::remove_var("PYTHON_RUNTIME_URL");
        env::remove_var("RUST_RUNTIME_URL");
        env::remove_var("RUNTIME_TIMEOUT_SECONDS");
        env::remove_var("MAX_SCRIPT_SIZE");
        env::remove_var("WASM_COMPILE_TIMEOUT_SECONDS");
    }

    #[test]
    fn test_config_from_env_with_defaults() {
        clear_env_vars();
        setup_env_vars();

        let config = Config::from_env().expect("Failed to load config");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, "postgres://user:pass@localhost:5432/testdb");
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.session_expiry_seconds, 3600);
        assert_eq!(config.runtime_config.nodejs_runtime_url, "http://localhost:8081");
        assert_eq!(config.runtime_config.python_runtime_url, "http://localhost:8082");
        assert_eq!(config.runtime_config.rust_runtime_url, "http://localhost:8083");
        assert_eq!(config.runtime_config.runtime_timeout_seconds, 30);
        assert_eq!(config.runtime_config.runtime_fallback_timeout_seconds, 15);
        assert_eq!(config.runtime_config.runtime_max_retries, 3);
        assert_eq!(config.runtime_config.max_script_size, 1048576);
        assert_eq!(config.runtime_config.wasm_compile_timeout_seconds, 60);
    }

    #[test]
    fn test_config_from_env_with_custom_values() {
        clear_env_vars();
        setup_env_vars();

        env::set_var("HOST", "127.0.0.1");
        env::set_var("PORT", "9090");
        env::set_var("SESSION_EXPIRY_SECONDS", "7200");
        env::set_var("RUNTIME_TIMEOUT_SECONDS", "60");
        env::set_var("MAX_SCRIPT_SIZE", "2097152");
        env::set_var("WASM_COMPILE_TIMEOUT_SECONDS", "120");

        let config = Config::from_env().expect("Failed to load config");

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.session_expiry_seconds, 7200);
        assert_eq!(config.runtime_config.runtime_timeout_seconds, 60);
        assert_eq!(config.runtime_config.max_script_size, 2097152);
        assert_eq!(config.runtime_config.wasm_compile_timeout_seconds, 120);
    }

    #[test]
    fn test_config_from_env_with_invalid_port() {
        clear_env_vars();
        setup_env_vars();

        env::set_var("PORT", "invalid");

        let result = Config::from_env();
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::Config(_)));
            assert!(err.to_string().contains("Invalid PORT"));
        }
    }

    #[test]
    fn test_config_from_env_with_missing_required_vars() {
        clear_env_vars();

        let result = Config::from_env();
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::Config(_)));
            assert!(err.to_string().contains("environment variable not set"));
        }
    }

    #[test]
    fn test_config_from_values() {
        let runtime_config = RuntimeConfig {
            nodejs_runtime_url: "http://nodejs:8080".to_string(),
            python_runtime_url: "http://python:8080".to_string(),
            rust_runtime_url: "http://rust:8080".to_string(),
            runtime_timeout_seconds: 45,
            runtime_fallback_timeout_seconds: 20,
            runtime_max_retries: 5,
            max_script_size: 2097152,
            wasm_compile_timeout_seconds: 90,
        };

        let config = Config::from_values(
            "localhost",
            8888,
            "postgres://test:test@db:5432/testdb",
            "redis://redis:6379",
            1800,
            runtime_config.clone(),
        );

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 8888);
        assert_eq!(config.database_url, "postgres://test:test@db:5432/testdb");
        assert_eq!(config.redis_url, "redis://redis:6379");
        assert_eq!(config.session_expiry_seconds, 1800);
        assert_eq!(config.runtime_config, runtime_config);
    }
}
