use crate::{
    session::DbPoolTrait,
    error::{Error, Result},
};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct DatabaseLogger<T: DbPoolTrait + ?Sized> {
    db_pool: Arc<T>,
    enabled: bool,
}

impl<T: DbPoolTrait + ?Sized> DatabaseLogger<T> {
    pub fn new(db_pool: Arc<T>, enabled: bool) -> Self {
        Self { db_pool, enabled }
    }

    pub async fn log_request(
        &self,
        request_id: &str,
        language_title: &str,
        client_ip: Option<&str>,
        user_id: Option<&str>,
        request_headers: Option<Value>,
        request_payload: Option<Value>,
        response_payload: Option<Value>,
        status_code: i32,
        duration_ms: i32,
        cached: bool,
        error_details: Option<Value>,
        runtime_metrics: Option<Value>,
    ) -> Result<()> {
        if !self.enabled {
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
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let request_payload_str = match &request_payload {
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let response_payload_str = match &response_payload {
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let error_details_str = match &error_details {
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let runtime_metrics_str = match &runtime_metrics {
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let result = (*self.db_pool).execute(
            query,
            &[
                &request_id,
                &language_title,
                &client_ip.unwrap_or(""),
                &user_id.unwrap_or(""),
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
    }

    pub async fn log_error(
        &self,
        request_log_id: &str,
        error_code: &str,
        error_message: &str,
        stack_trace: Option<&str>,
        context: Option<Value>,
    ) -> Result<()> {
        if !self.enabled {
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
            Some(v) => serde_json::to_string(v).unwrap_or("{}".to_string()),
            None => "{}".to_string(),
        };
        
        let result = (*self.db_pool).execute(
            query,
            &[
                &request_log_id,
                &error_code,
                &error_message,
                &stack_trace.unwrap_or(""),
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
            "test-request-id",
            "nodejs-test",
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
