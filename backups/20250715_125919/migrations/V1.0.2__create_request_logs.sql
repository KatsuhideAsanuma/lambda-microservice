CREATE TABLE IF NOT EXISTS public.request_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id VARCHAR(64) NOT NULL,
    language_title VARCHAR(128) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    client_ip VARCHAR(45),
    user_id VARCHAR(128),
    request_headers JSONB,
    request_payload JSONB,
    response_payload JSONB,
    status_code INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    cached BOOLEAN NOT NULL DEFAULT FALSE,
    error_details JSONB,
    runtime_metrics JSONB
);

CREATE INDEX IF NOT EXISTS idx_request_logs_timestamp ON public.request_logs (timestamp);
CREATE INDEX IF NOT EXISTS idx_request_logs_language_title ON public.request_logs (language_title);
CREATE INDEX IF NOT EXISTS idx_request_logs_status_code ON public.request_logs (status_code);
CREATE INDEX IF NOT EXISTS idx_request_logs_user_id ON public.request_logs (user_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_request_id ON public.request_logs (request_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_cached ON public.request_logs (cached);
