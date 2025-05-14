use crate::error::Result;
use deadpool_postgres::Client;
use tracing::{debug, info};

pub async fn initialize_database(client: &Client) -> Result<()> {
    info!("Initializing database schema...");
    
    debug!("Creating schemas...");
    client.batch_execute("
        CREATE SCHEMA IF NOT EXISTS meta;
        CREATE SCHEMA IF NOT EXISTS analytics;
    ").await?;
    
    debug!("Creating meta.sessions table...");
    client.batch_execute("
        CREATE TABLE IF NOT EXISTS meta.sessions (
            request_id VARCHAR(64) PRIMARY KEY,
            language_title VARCHAR(128) NOT NULL,
            user_id VARCHAR(128),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at TIMESTAMPTZ NOT NULL,
            last_executed_at TIMESTAMPTZ,
            execution_count INTEGER NOT NULL DEFAULT 0,
            status VARCHAR(16) NOT NULL DEFAULT 'active',
            context JSONB,
            script_content TEXT,
            script_hash VARCHAR(64),
            compiled_artifact BYTEA,
            compile_options JSONB,
            compile_status VARCHAR(16),
            compile_error TEXT,
            metadata JSONB
        );
        
        CREATE INDEX IF NOT EXISTS idx_sessions_language_title ON meta.sessions (language_title);
        CREATE INDEX IF NOT EXISTS idx_sessions_status ON meta.sessions (status);
        CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON meta.sessions (expires_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_script_hash ON meta.sessions (script_hash);
        CREATE INDEX IF NOT EXISTS idx_sessions_compile_status ON meta.sessions (compile_status);
    ").await?;
    
    debug!("Creating cleanup_expired_sessions function...");
    client.batch_execute("
        CREATE OR REPLACE FUNCTION meta.cleanup_expired_sessions()
        RETURNS INTEGER AS $$
        DECLARE
            deleted_count INTEGER;
        BEGIN
            DELETE FROM meta.sessions
            WHERE expires_at < NOW()
            AND status = 'active'
            RETURNING COUNT(*) INTO deleted_count;
            
            UPDATE meta.sessions
            SET status = 'expired'
            WHERE expires_at < NOW()
            AND status = 'active';
            
            RETURN deleted_count;
        END;
        $$ LANGUAGE plpgsql;
    ").await?;
    
    debug!("Creating update_session_on_execute function and trigger...");
    client.batch_execute("
        CREATE OR REPLACE FUNCTION meta.update_session_on_execute()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.last_executed_at = NOW();
            NEW.execution_count = NEW.execution_count + 1;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        
        DROP TRIGGER IF EXISTS update_session_on_execute ON meta.sessions;
        CREATE TRIGGER update_session_on_execute
        BEFORE UPDATE ON meta.sessions
        FOR EACH ROW
        WHEN (NEW.execution_count > OLD.execution_count)
        EXECUTE FUNCTION meta.update_session_on_execute();
    ").await?;
    
    debug!("Creating calculate_script_hash function and trigger...");
    client.batch_execute("
        CREATE OR REPLACE FUNCTION meta.calculate_script_hash()
        RETURNS TRIGGER AS $$
        BEGIN
            IF NEW.script_content IS NOT NULL THEN
                NEW.script_hash = encode(sha256(NEW.script_content::bytea), 'hex');
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        
        DROP TRIGGER IF EXISTS calculate_script_hash ON meta.sessions;
        CREATE TRIGGER calculate_script_hash
        BEFORE INSERT OR UPDATE OF script_content ON meta.sessions
        FOR EACH ROW
        EXECUTE FUNCTION meta.calculate_script_hash();
    ").await?;
    
    info!("Database schema initialization completed successfully");
    Ok(())
}
