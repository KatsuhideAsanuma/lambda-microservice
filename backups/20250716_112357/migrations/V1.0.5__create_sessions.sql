-- セッションテーブルの作成
CREATE TABLE IF NOT EXISTS meta.sessions (
    request_id VARCHAR(128) PRIMARY KEY,
    language_title VARCHAR(128) NOT NULL,
    user_id VARCHAR(128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_executed_at TIMESTAMPTZ,
    execution_count INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    context JSONB,
    script_content TEXT,
    script_hash VARCHAR(64),
    compiled_artifact BYTEA,
    compile_options JSONB,
    compile_status VARCHAR(32),
    compile_error TEXT,
    metadata JSONB
);

-- インデックスの作成
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON meta.sessions (expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_language_title ON meta.sessions (language_title);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON meta.sessions (user_id);

-- 期限切れセッションクリーンアップ関数
CREATE OR REPLACE FUNCTION meta.cleanup_expired_sessions()
RETURNS BIGINT AS $$
BEGIN
    DELETE FROM meta.sessions WHERE expires_at < NOW();
    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;
