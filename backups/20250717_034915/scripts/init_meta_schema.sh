#!/bin/bash
set -e

echo "Initializing meta schema and sessions table..."

if ! ./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh ps | grep -q "postgres.*healthy"; then
  echo "PostgreSQL container is not running or not healthy. Starting it..."
  ./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh up -d postgres
  echo "Waiting for PostgreSQL to be ready..."
  sleep 10
fi

./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh exec -T postgres psql -U postgres -d lambda_microservice -c "
CREATE SCHEMA IF NOT EXISTS meta;

CREATE TABLE IF NOT EXISTS meta.sessions (
    request_id VARCHAR(255) PRIMARY KEY,
    language_title VARCHAR(255) NOT NULL,
    user_id VARCHAR(255),
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    status VARCHAR(50) NOT NULL,
    context JSONB NOT NULL,
    script_content TEXT,
    compile_options JSONB
);
"

./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh exec -T postgres psql -U postgres -d lambda_microservice -c "
DROP FUNCTION IF EXISTS meta.cleanup_expired_sessions();
"

./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh exec -T postgres psql -U postgres -d lambda_microservice -c "
CREATE FUNCTION meta.cleanup_expired_sessions()
RETURNS INTEGER AS \$\$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM meta.sessions
    WHERE expires_at < NOW();
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
\$\$ LANGUAGE plpgsql;
"

echo "✅ Meta schema and sessions table initialized successfully!"
