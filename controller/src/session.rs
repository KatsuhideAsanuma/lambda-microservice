
use crate::{
    cache::RedisPool,
    database::PostgresPool,
    error::{Error, Result},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            script_content,
            script_hash,
            compiled_artifact: None,
            compile_options,
            compile_status: script_content.as_ref().map(|_| "pending".to_string()),
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
}

pub struct SessionManager {
    db_pool: PostgresPool,
    redis_pool: RedisPool,
    session_expiry_seconds: u64,
}

impl SessionManager {
    pub fn new(db_pool: PostgresPool, redis_pool: RedisPool, session_expiry_seconds: u64) -> Self {
        Self {
            db_pool,
            redis_pool,
            session_expiry_seconds,
        }
    }

    pub async fn create_session(
        &self,
        language_title: String,
        user_id: Option<String>,
        context: serde_json::Value,
        script_content: Option<String>,
        compile_options: Option<serde_json::Value>,
    ) -> Result<Session> {
        let session = Session::new(
            language_title,
            user_id,
            context,
            script_content,
            compile_options,
            self.session_expiry_seconds,
        );

        self.redis_pool
            .set_ex(
                &format!("session:{}", session.request_id),
                &session,
                self.session_expiry_seconds,
            )
            .await?;

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

    pub async fn get_session(&self, request_id: &str) -> Result<Option<Session>> {
        let redis_key = format!("session:{}", request_id);
        if let Some(session) = self.redis_pool.get::<Session>(&redis_key).await? {
            if session.is_expired() {
                self.expire_session(request_id).await?;
                return Ok(None);
            }
            return Ok(Some(session));
        }

        let query = r#"
            SELECT
                request_id, language_title, user_id, created_at, expires_at,
                last_executed_at, execution_count, status, context,
                script_content, script_hash, compiled_artifact, compile_options,
                compile_status, compile_error, metadata
            FROM meta.sessions
            WHERE request_id = $1
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

            if session.is_expired() {
                self.expire_session(request_id).await?;
                return Ok(None);
            }

            self.redis_pool
                .set_ex(
                    &redis_key,
                    &session,
                    self.session_expiry_seconds,
                )
                .await?;

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub async fn update_session(&self, session: &Session) -> Result<()> {
        self.redis_pool
            .set_ex(
                &format!("session:{}", session.request_id),
                session,
                self.session_expiry_seconds,
            )
            .await?;

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

    pub async fn expire_session(&self, request_id: &str) -> Result<()> {
        self.redis_pool
            .del(&format!("session:{}", request_id))
            .await?;

        let query = r#"
            UPDATE meta.sessions
            SET status = 'expired'
            WHERE request_id = $1
        "#;

        self.db_pool.execute(query, &[&request_id]).await?;

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let query = r#"
            SELECT meta.cleanup_expired_sessions()
        "#;

        let row = self.db_pool.query_one(query, &[]).await?;
        let count: i64 = row.get(0);

        Ok(count as u64)
    }
}
