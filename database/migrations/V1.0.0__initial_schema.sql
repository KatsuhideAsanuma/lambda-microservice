CREATE SCHEMA IF NOT EXISTS meta;

CREATE TABLE IF NOT EXISTS meta.schema_version (
    id SERIAL PRIMARY KEY,
    version VARCHAR(32) NOT NULL,
    description TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied_by VARCHAR(128) NOT NULL
);

INSERT INTO meta.schema_version (version, description, applied_by)
VALUES ('1.0.0', 'Initial schema creation', 'system');
