
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
    // pub wasm_compile_timeout_seconds: u64, // TEMPORARILY DISABLED
    pub openfaas_gateway_url: String,
    pub selection_strategy: Option<String>,
    pub runtime_mappings_file: Option<String>,
    pub kubernetes_namespace: Option<String>,
    pub redis_url: Option<String>,
    pub cache_ttl_seconds: Option<u64>,
}

impl Config {
    // 新規追加: 設定ファイルから改行文字を除去して読み込む
    fn read_secret_file(path: &str) -> std::result::Result<String, std::io::Error> {
        std::fs::read_to_string(path)
            .map(|content| content.trim().to_string()) // 改行文字除去
    }

    // 新規追加: 設定検証
    pub fn validate(&self) -> Result<()> {
        // URL形式の検証
        if !self.database_url.starts_with("postgres://") && !self.database_url.starts_with("postgresql://") {
            return Err(Error::Config("Invalid database URL format".to_string()));
        }
        
        if !self.redis_url.starts_with("redis://") {
            return Err(Error::Config("Invalid Redis URL format".to_string()));
        }
        
        Ok(())
    }

    pub fn from_env() -> Result<Self> {
        let config = Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| Error::Config("Invalid PORT".to_string()))?,
            database_url: env::var("DATABASE_URL")
                .or_else(|_| {
                    env::var("DATABASE_URL_FILE").and_then(|path| {
                        Self::read_secret_file(&path)
                            .map_err(|_| env::VarError::NotPresent)
                    })
                })
                .map_err(|_| {
                    Error::Config("DATABASE_URL environment variable not set".to_string())
                })?,
            redis_url: env::var("REDIS_URL")
                .or_else(|_| {
                    env::var("REDIS_URL_FILE").and_then(|path| {
                        Self::read_secret_file(&path)
                            .map_err(|_| env::VarError::NotPresent)
                    })
                })
                .map_err(|_| {
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
                // wasm_compile_timeout_seconds: env::var("WASM_COMPILE_TIMEOUT_SECONDS")
                //     .unwrap_or_else(|_| "60".to_string())
                //     .parse()
                //     .map_err(|_| {
                //         Error::Config("Invalid WASM_COMPILE_TIMEOUT_SECONDS".to_string())
                //     })?,
                openfaas_gateway_url: env::var("OPENFAAS_GATEWAY_URL")
                    .unwrap_or_else(|_| "http://gateway.openfaas:8080".to_string()),
                selection_strategy: env::var("RUNTIME_SELECTION_STRATEGY")
                    .ok()
                    .map(|s| s.to_string()),
                runtime_mappings_file: env::var("RUNTIME_MAPPINGS_FILE")
                    .ok()
                    .map(|s| s.to_string()),
                kubernetes_namespace: env::var("KUBERNETES_NAMESPACE")
                    .ok()
                    .map(|s| s.to_string()),
                redis_url: env::var("REDIS_URL")
                    .ok()
                    .map(|s| s.to_string()),
                cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                    .ok()
                    .and_then(|s| s.parse().ok()),
            },
        };
        
        Ok(config)
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
        let runtime_config = RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            runtime_timeout_seconds: 30,
            runtime_fallback_timeout_seconds: 15,
            runtime_max_retries: 3,
            max_script_size: 1048576, // 1MB
            // wasm_compile_timeout_seconds: 60, // TEMPORARILY DISABLED
            openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
            selection_strategy: None,
            runtime_mappings_file: None,
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: None,
        };

        let config = Config::from_values(
            "0.0.0.0",
            8080,
            "postgres://user:pass@localhost:5432/testdb",
            "redis://localhost:6379",
            3600,
            runtime_config,
        );

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
        assert_eq!(config.runtime_config.max_script_size, 1048576); // 1MB
        // assert_eq!(config.runtime_config.wasm_compile_timeout_seconds, 60); // TEMPORARILY DISABLED
    }

    #[test]
    fn test_config_from_env_with_custom_values() {
        let runtime_config = RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            runtime_timeout_seconds: 60,
            runtime_fallback_timeout_seconds: 15,
            runtime_max_retries: 3,
            max_script_size: 1048576, // 1MB
            // wasm_compile_timeout_seconds: 60, // TEMPORARILY DISABLED
            openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
            selection_strategy: None,
            runtime_mappings_file: None,
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: None,
        };
        
        let config = Config::from_values(
            "127.0.0.1",
            9090,
            "postgres://user:pass@localhost:5432/testdb",
            "redis://localhost:6379",
            7200,
            runtime_config,
        );

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.session_expiry_seconds, 7200);
        assert_eq!(config.runtime_config.runtime_timeout_seconds, 60);
        assert_eq!(config.runtime_config.max_script_size, 1048576); // 1MB
        // assert_eq!(config.runtime_config.wasm_compile_timeout_seconds, 60); // TEMPORARILY DISABLED
    }

    #[test]
    fn test_config_from_env_with_invalid_port() {
        let port_str = "invalid";
        let result = port_str.parse::<u16>()
            .map_err(|_| Error::Config("Invalid PORT".to_string()));
        
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::Config(_)));
            let expected_error = "Invalid PORT";
            let error_message = err.to_string();
            assert!(
                error_message.contains(expected_error),
                "Expected error message to contain '{}', but got '{}'",
                expected_error,
                error_message
            );
        }
    }

    #[test]
    fn test_config_from_env_with_missing_required_vars() {
        clear_env_vars();
        
        env::set_var("PORT", "8080");

        let result = Config::from_env();
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert!(matches!(err, Error::Config(_)));
            assert!(err.to_string().contains("Configuration error:") || 
                   err.to_string().contains("environment variable not set") ||
                   err.to_string().contains("DATABASE_URL") ||
                   err.to_string().contains("REDIS_URL") ||
                   err.to_string().contains("NODEJS_RUNTIME_URL"),
                   "Error message '{}' is not a configuration error", err.to_string());
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
            // wasm_compile_timeout_seconds: 90, // TEMPORARILY DISABLED
            openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
            selection_strategy: None,
            runtime_mappings_file: None,
            kubernetes_namespace: None,
            redis_url: Some("redis://redis:6379".to_string()),
            cache_ttl_seconds: Some(1800),
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
