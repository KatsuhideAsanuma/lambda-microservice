CREATE TABLE IF NOT EXISTS meta.functions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    language VARCHAR(32) NOT NULL,
    title VARCHAR(64) NOT NULL,
    language_title VARCHAR(128) NOT NULL UNIQUE,
    description TEXT,
    schema_definition JSONB,
    examples JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(128),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    version VARCHAR(32) NOT NULL DEFAULT '1.0.0',
    tags VARCHAR(64)[]
);

CREATE INDEX IF NOT EXISTS idx_functions_language ON meta.functions (language);
CREATE INDEX IF NOT EXISTS idx_functions_language_title ON meta.functions (language_title);
CREATE INDEX IF NOT EXISTS idx_functions_is_active ON meta.functions (is_active);
CREATE INDEX IF NOT EXISTS idx_functions_tags ON meta.functions USING GIN (tags);

CREATE TABLE IF NOT EXISTS meta.scripts (
    function_id UUID PRIMARY KEY REFERENCES meta.functions(id),
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
