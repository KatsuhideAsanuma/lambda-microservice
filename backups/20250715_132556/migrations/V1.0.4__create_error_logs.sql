CREATE TABLE IF NOT EXISTS public.error_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_log_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_code VARCHAR(64) NOT NULL,
    error_message TEXT NOT NULL,
    stack_trace TEXT,
    context JSONB
);

CREATE INDEX IF NOT EXISTS idx_error_logs_request_log_id ON public.error_logs (request_log_id);
CREATE INDEX IF NOT EXISTS idx_error_logs_error_code ON public.error_logs (error_code);
CREATE INDEX IF NOT EXISTS idx_error_logs_timestamp ON public.error_logs (timestamp);

ALTER TABLE public.error_logs 
    ADD CONSTRAINT fk_error_logs_request_log_id 
    FOREIGN KEY (request_log_id) 
    REFERENCES public.request_logs(id);
