use crate::error::Result;
use crate::logger::DatabaseLoggerTrait;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockDatabaseLogger {
    log_error_result: Arc<Mutex<Result<()>>>,
}

impl MockDatabaseLogger {
    pub fn new() -> Self {
        Self {
            log_error_result: Arc::new(Mutex::new(Ok(()))),
        }
    }

    pub fn with_log_error_result(mut self, result: Result<()>) -> Self {
        self.log_error_result = Arc::new(Mutex::new(result));
        self
    }
}

#[async_trait]
impl DatabaseLoggerTrait for MockDatabaseLogger {
    async fn log_error(&self, _error: &str, _context: Option<&str>) -> Result<()> {
        self.log_error_result.lock().await.clone()
    }
}
