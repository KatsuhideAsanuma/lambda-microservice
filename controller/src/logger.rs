use crate::{
    session::DbPoolTrait,
    error::{Error, Result},
};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error};
use std::future::Future;
use std::pin::Pin;

pub trait DatabaseLoggerTrait: Send + Sync {
    fn log_request(
        &self,
        request_id: String,
        language_title: String,
        client_ip: Option<String>,
        user_id: Option<String>,
        request_headers: Option<Value>,
        request_payload: Option<Value>,
        response_payload: Option<Value>,
        status_code: i32,
        duration_ms: i64,
        cached: bool,
        error_details: Option<Value>,
        runtime_metrics: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
    
    fn log_error(
        &self,
        request_log_id: String,
        error_code: String,
        error_message: String,
        stack_trace: Option<String>,
        context: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

pub struct DatabaseLogger<T: DbPoolTrait + ?Sized> {
    db_pool: Arc<T>,
    enabled: bool,
}

impl<T: DbPoolTrait + ?Sized> DatabaseLogger<T> {
    pub fn new(db_pool: Arc<T>, enabled: bool) -> Self {
        Self { db_pool, enabled }
    }
}

impl<T: DbPoolTrait + ?Sized + 'static> DatabaseLoggerTrait for DatabaseLogger<T> {
    fn log_request(
        &self,
        request_id: String,
        language_title: String,
        client_ip: Option<String>,
        user_id: Option<String>,
        request_headers: Option<Value>,
        request_payload: Option<Value>,
        response_payload: Option<Value>,
        status_code: i32,
        duration_ms: i64,
        cached: bool,
        error_details: Option<Value>,
        runtime_metrics: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let db_pool = self.db_pool.clone();
        let enabled = self.enabled;
        Box::pin(async move {
            if !enabled {
                return Ok(());
            }

            let query = r#"
                INSERT INTO public.request_logs (
                    request_id, language_title, client_ip, user_id, 
                    request_headers, request_payload, response_payload, 
                    status_code, duration_ms, cached, error_details, runtime_metrics
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
                )
            "#;
            
            let request_headers_str = match &request_headers {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let request_payload_str = match &request_payload {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let response_payload_str = match &response_payload {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let error_details_str = match &error_details {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let runtime_metrics_str = match &runtime_metrics {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let result = (*db_pool).execute(
                query,
                &[
                    &request_id,
                    &language_title,
                    &client_ip.unwrap_or_else(|| "".to_string()),
                    &user_id.unwrap_or_else(|| "".to_string()),
                    &request_headers_str,
                    &request_payload_str,
                    &response_payload_str,
                    &status_code,
                    &duration_ms,
                    &cached,
                    &error_details_str,
                    &runtime_metrics_str,
                ],
            ).await;
                
            match result {
                Ok(_) => {
                    debug!("Successfully logged request {}", request_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to log request {}: {}", request_id, e);
                    Err(Error::Database(e.to_string()))
                }
            }
        })
    }

    fn log_error(
        &self,
        request_log_id: String,
        error_code: String,
        error_message: String,
        stack_trace: Option<String>,
        context: Option<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let db_pool = self.db_pool.clone();
        let enabled = self.enabled;
        Box::pin(async move {
            if !enabled {
                return Ok(());
            }

            let query = r#"
                INSERT INTO public.error_logs (
                    request_log_id, error_code, error_message, stack_trace, context
                ) VALUES (
                    $1, $2, $3, $4, $5
                )
            "#;
            
            let context_str = match &context {
                Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                None => "{}".to_string(),
            };
            
            let result = (*db_pool).execute(
                query,
                &[
                    &request_log_id,
                    &error_code,
                    &error_message,
                    &stack_trace.unwrap_or_else(|| "".to_string()),
                    &context_str,
                ],
            ).await;
                
            match result {
                Ok(_) => {
                    debug!("Successfully logged error for request {}", request_log_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to log error for request {}: {}", request_log_id, e);
                    Err(Error::Database(e.to_string()))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::tests::MockPostgresPool;
    
    #[tokio::test]
    async fn test_log_request_disabled() {
        let pool = Arc::new(MockPostgresPool::new());
        let logger = DatabaseLogger::new(pool, false);
        
        let result = logger.log_request(
            "test-request-id".to_string(),
            "nodejs-test".to_string(),
            None,
            None,
            None,
            None,
            None,
            200,
            100,
            false,
            None,
            None,
        ).await;
        
        assert!(result.is_ok());
    }
}
