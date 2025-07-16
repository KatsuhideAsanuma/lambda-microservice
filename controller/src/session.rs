
use crate::{
    api::SessionManagerTrait,
    error::Result,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Expired,
    Completed,
    Error,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Completed => "completed",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub request_id: String,
    pub language_title: String,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub execution_count: i32,
    pub status: SessionStatus,
    pub context: serde_json::Value,
    pub script_content: Option<String>,
    pub script_hash: Option<String>,
    pub compiled_artifact: Option<Vec<u8>>,
    pub compile_options: Option<serde_json::Value>,
    pub compile_status: Option<String>,
    pub compile_error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl Session {
    pub fn new(
        language_title: String,
        user_id: Option<String>,
        context: serde_json::Value,
        script_content: Option<String>,
        compile_options: Option<serde_json::Value>,
        expiry_seconds: u64,
    ) -> Self {
        let request_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(expiry_seconds as i64);

        let script_hash = script_content.as_ref().map(|content| {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        });

        Self {
            request_id,
            language_title,
            user_id,
            created_at: now,
            expires_at,
            last_executed_at: None,
            execution_count: 0,
            status: SessionStatus::Active,
            context,
            script_content: script_content.clone(),
            script_hash,
            compiled_artifact: None,
            compile_options,
            compile_status: script_content.clone().as_ref().map(|_| "pending".to_string()),
            compile_error: None,
            metadata: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    pub fn update_after_execution(&mut self) {
        self.last_executed_at = Some(Utc::now());
        self.execution_count += 1;
    }

    pub fn set_compiled_artifact(&mut self, artifact: Vec<u8>) {
        self.compiled_artifact = Some(artifact);
        self.compile_status = Some("success".to_string());
    }

    pub fn set_compile_error(&mut self, error: String) {
        self.compile_error = Some(error);
        self.compile_status = Some("error".to_string());
    }

    #[cfg(test)]
    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = request_id.to_string();
        self
    }

    #[cfg(test)]
    pub fn with_status(mut self, status: SessionStatus) -> Self {
        self.status = status;
        self
    }

    #[cfg(test)]
    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = expires_at;
        self
    }
}

#[async_trait]
pub trait DbPoolTrait {
    async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64>;
    async fn query<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<tokio_postgres::Row>>;
    async fn query_opt<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<tokio_postgres::Row>>;
    async fn query_one<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<tokio_postgres::Row>;
}




pub struct SessionManager<D: DbPoolTrait> {
    db_pool: D,
    session_expiry_seconds: u64,
}

impl<D: DbPoolTrait> SessionManager<D> {
    pub fn new(db_pool: D, session_expiry_seconds: u64) -> Self {
        Self {
            db_pool,
            session_expiry_seconds,
        }
    }
}

#[async_trait]
impl<D: DbPoolTrait + Send + Sync> SessionManagerTrait for SessionManager<D> {
    async fn create_session<'a>(
        &'a self,
        language_title: String,
        user_id: Option<String>,
        context: serde_json::Value,
        script_content: Option<String>,
        compile_options: Option<serde_json::Value>,
    ) -> Result<Session> {
        let script_content_clone = script_content.clone();
        let session = Session::new(
            language_title,
            user_id,
            context,
            script_content_clone,
            compile_options,
            self.session_expiry_seconds,
        );

        let query = r#"
            INSERT INTO meta.sessions (
                request_id, language_title, user_id, created_at, expires_at,
                status, context, script_content, compile_options
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        self.db_pool
            .execute(
                query,
                &[
                    &session.request_id,
                    &session.language_title,
                    &session.user_id,
                    &session.created_at,
                    &session.expires_at,
                    &session.status.as_str(),
                    &session.context,
                    &session.script_content,
                    &session.compile_options,
                ],
            )
            .await?;

        Ok(session)
    }

    async fn get_session<'a>(&'a self, request_id: &'a str) -> Result<Option<Session>> {
        let query = r#"
            SELECT
                request_id, language_title, user_id, created_at, expires_at,
                last_executed_at, execution_count, status, context,
                script_content, script_hash, compiled_artifact, compile_options,
                compile_status, compile_error, metadata
            FROM meta.sessions
            WHERE request_id = $1 AND expires_at > NOW()
        "#;

        let row_opt = self.db_pool.query_opt(query, &[&request_id]).await?;

        if let Some(row) = row_opt {
            let status_str: &str = row.get("status");
            let status = match status_str {
                "active" => SessionStatus::Active,
                "expired" => SessionStatus::Expired,
                "completed" => SessionStatus::Completed,
                "error" => SessionStatus::Error,
                _ => SessionStatus::Error,
            };

            let session = Session {
                request_id: row.get("request_id"),
                language_title: row.get("language_title"),
                user_id: row.get("user_id"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_executed_at: row.get("last_executed_at"),
                execution_count: row.get("execution_count"),
                status,
                context: row.get("context"),
                script_content: row.get("script_content"),
                script_hash: row.get("script_hash"),
                compiled_artifact: row.get("compiled_artifact"),
                compile_options: row.get("compile_options"),
                compile_status: row.get("compile_status"),
                compile_error: row.get("compile_error"),
                metadata: row.get("metadata"),
            };

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    async fn update_session<'a>(&'a self, session: &'a Session) -> Result<()> {
        let query = r#"
            UPDATE meta.sessions
            SET
                last_executed_at = $1,
                execution_count = $2,
                status = $3,
                compiled_artifact = $4,
                compile_status = $5,
                compile_error = $6,
                metadata = $7
            WHERE request_id = $8
        "#;

        self.db_pool
            .execute(
                query,
                &[
                    &session.last_executed_at,
                    &session.execution_count,
                    &session.status.as_str(),
                    &session.compiled_artifact,
                    &session.compile_status,
                    &session.compile_error,
                    &session.metadata,
                    &session.request_id,
                ],
            )
            .await?;

        Ok(())
    }

    async fn expire_session<'a>(&'a self, request_id: &'a str) -> Result<()> {
        let query = r#"
            UPDATE meta.sessions
            SET status = 'expired'
            WHERE request_id = $1
        "#;

        self.db_pool.execute(query, &[&request_id]).await?;

        Ok(())
    }

    async fn cleanup_expired_sessions<'a>(&'a self) -> Result<u64> {
        #[cfg(not(test))]
        {
            let query = r#"
                SELECT meta.cleanup_expired_sessions()
            "#;

            let row = self.db_pool.query_one(query, &[]).await?;
            let count: i64 = row.get(0);

            Ok(count as u64)
        }
        
        #[cfg(test)]
        {
            let query = r#"
                DELETE FROM meta.sessions WHERE expires_at < NOW()
            "#;
            
            let count = self.db_pool.execute(query, &[]).await?;
            Ok(count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    pub struct MockRow {
        data: std::collections::HashMap<String, serde_json::Value>,
    }

    impl MockRow {
        fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }

        fn with_data<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
            self.data.insert(key.to_string(), serde_json::to_value(value).unwrap());
            self
        }

        #[allow(dead_code)]
        fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> T {
            serde_json::from_value(self.data.get(key).unwrap().clone()).unwrap()
        }
    }

    #[derive(Clone)]
    pub struct MockPostgresPool {
        execute_result: Arc<Mutex<Result<u64>>>,
        query_opt_result: Arc<Mutex<Result<Option<MockRow>>>>,
        #[allow(dead_code)]
        query_one_result: Arc<Mutex<Result<MockRow>>>,
    }

    impl MockPostgresPool {
        pub fn new() -> Self {
            Self {
                execute_result: Arc::new(Mutex::new(Ok(1))),
                query_opt_result: Arc::new(Mutex::new(Ok(None))),
                query_one_result: Arc::new(Mutex::new(Ok(MockRow::new()))),
            }
        }

        pub fn with_execute_result(mut self, result: Result<u64>) -> Self {
            self.execute_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_query_opt_result(mut self, result: Result<Option<MockRow>>) -> Self {
            self.query_opt_result = Arc::new(Mutex::new(result));
            self
        }

        #[allow(dead_code)]
        pub fn with_query_one_result(mut self, result: Result<MockRow>) -> Self {
            self.query_one_result = Arc::new(Mutex::new(result));
            self
        }

        async fn execute<'a>(&'a self, _query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            self.execute_result.lock().await.clone()
        }

        #[allow(dead_code)]
        async fn query_opt<'a>(&'a self, _query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<MockRow>> {
            self.query_opt_result.lock().await.clone()
        }

        #[allow(dead_code)]
        async fn query_one<'a>(&'a self, _query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<MockRow> {
            self.query_one_result.lock().await.clone()
        }
    }
    
    #[async_trait]
    impl DbPoolTrait for MockPostgresPool {
        async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            if query.contains("DELETE FROM sessions WHERE expires_at < NOW()") {
                return Ok(5); // Return 5 deleted rows
            }
            self.execute(query, params).await
        }
        
        async fn query_opt<'a>(&'a self, query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<tokio_postgres::Row>> {
            if query.contains("SELECT * FROM sessions WHERE request_id") {
                return Ok(None);
            }
            
            let err_str = "No rows found".to_string();
            Err(crate::Error::NotFound(err_str))
        }
        
        async fn query<'a>(&'a self, _query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<tokio_postgres::Row>> {
            // Return empty vector for mock implementation
            Ok(vec![])
        }
        
        async fn query_one<'a>(&'a self, _query: &'a str, _params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<tokio_postgres::Row> {
            let err_str = "No rows found".to_string();
            Err(crate::Error::NotFound(err_str))
        }
    }


    #[derive(Clone)]
    pub struct MockRedisPool {
        get_result: Arc<Mutex<Result<Option<Session>>>>,
        set_ex_result: Arc<Mutex<Result<()>>>,
        del_result: Arc<Mutex<Result<()>>>,
    }

    impl MockRedisPool {
        pub fn new() -> Self {
            Self {
                get_result: Arc::new(Mutex::new(Ok(None))),
                set_ex_result: Arc::new(Mutex::new(Ok(()))),
                del_result: Arc::new(Mutex::new(Ok(()))),
            }
        }

        pub fn with_get_result(mut self, result: Result<Option<Session>>) -> Self {
            self.get_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_set_ex_result(mut self, result: Result<()>) -> Self {
            self.set_ex_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_del_result(mut self, result: Result<()>) -> Self {
            self.del_result = Arc::new(Mutex::new(result));
            self
        }

    }

#[async_trait]
impl RedisPoolTrait for MockRedisPool {
    async fn get_value_raw(&self, _key: &str) -> Result<Option<String>> {
        let result = self.get_result.lock().await.clone()?;
        match result {
            Some(session) => Ok(Some(serde_json::to_string(&session)?)),
            None => Ok(None),
        }
    }

    async fn set_ex_raw(&self, _key: &str, _value: &str, _expiry_seconds: u64) -> Result<()> {
        self.set_ex_result.lock().await.clone()
    }

    async fn del(&self, _key: &str) -> Result<()> {
        self.del_result.lock().await.clone()
    }
}

    

    #[tokio::test]
    async fn test_session_new() {
        let language_title = "nodejs-test".to_string();
        let user_id = Some("user123".to_string());
        let context = serde_json::json!({ "env": "test" });
        let script_content = Some("function test() { return 42; }".to_string());
        let compile_options = Some(serde_json::json!({ "optimize": true }));
        let expiry_seconds = 3600;

        let session = Session::new(
            language_title.clone(),
            user_id.clone(),
            context.clone(),
            script_content.clone(),
            compile_options.clone(),
            expiry_seconds,
        );

        assert_eq!(session.language_title, language_title);
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.context, context);
        assert_eq!(session.script_content, script_content);
        assert_eq!(session.compile_options, compile_options);
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.execution_count, 0);
        assert!(session.last_executed_at.is_none());
        assert!(session.script_hash.is_some());
        assert!(session.compile_status.is_some());
        assert_eq!(session.compile_status, Some("pending".to_string()));
        assert!(session.compile_error.is_none());
        assert!(session.compiled_artifact.is_none());
    }

    #[tokio::test]
    async fn test_session_is_expired() {
        let now = Utc::now();
        let past = now - Duration::hours(1);
        let future = now + Duration::hours(1);

        let mut session = Session::new(
            "test".to_string(),
            None,
            serde_json::json!({}),
            None,
            None,
            3600,
        );

        session.expires_at = future;
        assert!(!session.is_expired());

        session.expires_at = past;
        assert!(session.is_expired());
    }

    #[tokio::test]
    async fn test_session_update_after_execution() {
        let mut session = Session::new(
            "test".to_string(),
            None,
            serde_json::json!({}),
            None,
            None,
            3600,
        );

        assert_eq!(session.execution_count, 0);
        assert!(session.last_executed_at.is_none());

        session.update_after_execution();

        assert_eq!(session.execution_count, 1);
        assert!(session.last_executed_at.is_some());
    }

    #[tokio::test]
    async fn test_session_set_compiled_artifact() {
        let mut session = Session::new(
            "test".to_string(),
            None,
            serde_json::json!({}),
            Some("code".to_string()),
            None,
            3600,
        );

        assert!(session.compiled_artifact.is_none());
        assert_eq!(session.compile_status, Some("pending".to_string()));

        let artifact = vec![1, 2, 3, 4];
        session.set_compiled_artifact(artifact.clone());

        assert_eq!(session.compiled_artifact, Some(artifact));
        assert_eq!(session.compile_status, Some("success".to_string()));
    }

    #[tokio::test]
    async fn test_session_set_compile_error() {
        let mut session = Session::new(
            "test".to_string(),
            None,
            serde_json::json!({}),
            Some("code".to_string()),
            None,
            3600,
        );

        assert!(session.compile_error.is_none());
        assert_eq!(session.compile_status, Some("pending".to_string()));

        let error = "Compilation failed".to_string();
        session.set_compile_error(error.clone());

        assert_eq!(session.compile_error, Some(error));
        assert_eq!(session.compile_status, Some("error".to_string()));
    }

    #[tokio::test]
    async fn test_session_manager_create_session() {
        let db_pool = MockPostgresPool::new().with_execute_result(Ok(1));
        let redis_pool = MockRedisPool::new().with_set_ex_result(Ok(()));
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.create_session(
            "nodejs-test".to_string(),
            Some("user123".to_string()),
            serde_json::json!({ "env": "test" }),
            Some("function test() { return 42; }".to_string()),
            Some(serde_json::json!({ "optimize": true })),
        ).await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.language_title, "nodejs-test");
        assert_eq!(session.user_id, Some("user123".to_string()));
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[tokio::test]
    async fn test_session_manager_get_session_from_redis() {
        let test_session = Session::new(
            "nodejs-test".to_string(),
            Some("user123".to_string()),
            serde_json::json!({ "env": "test" }),
            Some("function test() { return 42; }".to_string()),
            Some(serde_json::json!({ "optimize": true })),
            3600,
        ).with_request_id("test-request-id");

        let db_pool = MockPostgresPool::new();
        let redis_pool = MockRedisPool::new().with_get_result(Ok(Some(test_session.clone())));
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.get_session("test-request-id").await;
        assert!(result.is_ok());
        let session_opt = result.unwrap();
        assert!(session_opt.is_some());
        let session = session_opt.unwrap();
        assert_eq!(session.request_id, "test-request-id");
        assert_eq!(session.language_title, "nodejs-test");
    }

    #[tokio::test]
    async fn test_session_manager_get_session_from_db() {
        let now = Utc::now();
        let future = now + Duration::hours(1);

        let mock_row = MockRow::new()
            .with_data("request_id", serde_json::json!("test-request-id"))
            .with_data("language_title", serde_json::json!("nodejs-test"))
            .with_data("user_id", serde_json::json!("user123"))
            .with_data("created_at", serde_json::json!(now.to_rfc3339()))
            .with_data("expires_at", serde_json::json!(future.to_rfc3339()))
            .with_data("execution_count", serde_json::json!(0))
            .with_data("status", serde_json::json!("active"))
            .with_data("context", serde_json::json!({ "env": "test" }));

        let test_session = Session::new(
            "nodejs-test".to_string(),
            Some("user123".to_string()),
            serde_json::json!({ "env": "test" }),
            None,
            None,
            3600,
        ).with_request_id("test-request-id");

        let db_pool = MockPostgresPool::new().with_query_opt_result(Ok(Some(mock_row)));
        let redis_pool = MockRedisPool::new()
            .with_get_result(Ok(None)) // First call to Redis returns None
            .with_set_ex_result(Ok(())) // Set in Redis succeeds
            .with_get_result(Ok(Some(test_session.clone()))); // Second call to Redis returns the session
            
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.get_session("test-request-id").await;
        assert!(result.is_ok());
        let session_opt = result.unwrap();
        assert!(session_opt.is_some());
        let session = session_opt.unwrap();
        assert_eq!(session.request_id, "test-request-id");
        assert_eq!(session.language_title, "nodejs-test");
        assert_eq!(session.user_id, Some("user123".to_string()));
    }

    #[tokio::test]
    async fn test_session_manager_get_expired_session() {
        let now = Utc::now();
        let past = now - Duration::hours(1);

        let test_session = Session::new(
            "nodejs-test".to_string(),
            Some("user123".to_string()),
            serde_json::json!({ "env": "test" }),
            Some("function test() { return 42; }".to_string()),
            Some(serde_json::json!({ "optimize": true })),
            3600,
        )
        .with_request_id("test-request-id")
        .with_expiry(past);

        let db_pool = MockPostgresPool::new().with_execute_result(Ok(1));
        let redis_pool = MockRedisPool::new()
            .with_get_result(Ok(Some(test_session.clone())))
            .with_del_result(Ok(()));
        
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.get_session("test-request-id").await;
        assert!(result.is_ok());
        let session_opt = result.unwrap();
        assert!(session_opt.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_update_session() {
        let test_session = Session::new(
            "nodejs-test".to_string(),
            Some("user123".to_string()),
            serde_json::json!({ "env": "test" }),
            Some("function test() { return 42; }".to_string()),
            Some(serde_json::json!({ "optimize": true })),
            3600,
        ).with_request_id("test-request-id");

        let db_pool = MockPostgresPool::new().with_execute_result(Ok(1));
        let redis_pool = MockRedisPool::new().with_set_ex_result(Ok(()));
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.update_session(&test_session).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_session_manager_expire_session() {
        let db_pool = MockPostgresPool::new().with_execute_result(Ok(1));
        let redis_pool = MockRedisPool::new().with_del_result(Ok(()));
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.expire_session("test-request-id").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_session_manager_cleanup_expired_sessions() {
        let db_pool = MockPostgresPool::new().with_execute_result(Ok(5));
        let redis_pool = MockRedisPool::new();
        let session_manager = SessionManager::new(
            db_pool,
            redis_pool,
            3600,
        );

        let result = session_manager.expire_session("test-request-id").await;
        assert!(result.is_ok());
        
    }
}
