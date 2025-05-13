# データベーススキーマ設計

## 1. 概要

lambda-microservice基盤で使用するPostgreSQLデータベースのスキーマ設計を定義します。このデータベースは主に処理ログの永続化、メタデータの管理、および分析用途に使用されます。

## 2. データベース構成

### 2.1 データベース

| 名前 | 説明 | 文字セット | 照合順序 |
|------|------|------------|---------|
| lambda_logs | 処理ログ保存用 | UTF-8 | C.UTF-8 |
| lambda_meta | メタデータ管理用 | UTF-8 | C.UTF-8 |

### 2.2 スキーマ

| 名前 | 説明 |
|------|------|
| public | デフォルトスキーマ（ログテーブル） |
| meta | メタデータ管理用スキーマ |
| analytics | 分析用ビュー・集計テーブル |

## 3. テーブル定義

### 3.1 request_logs テーブル

処理リクエストとレスポンスのログを保存するメインテーブル

```sql
CREATE TABLE public.request_logs (
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

-- インデックス
CREATE INDEX idx_request_logs_timestamp ON public.request_logs (timestamp);
CREATE INDEX idx_request_logs_language_title ON public.request_logs (language_title);
CREATE INDEX idx_request_logs_status_code ON public.request_logs (status_code);
CREATE INDEX idx_request_logs_user_id ON public.request_logs (user_id);
CREATE INDEX idx_request_logs_request_id ON public.request_logs (request_id);
CREATE INDEX idx_request_logs_cached ON public.request_logs (cached);
```

**パーティショニング（オプション）**:

大量のログデータを効率的に管理するために、日付ベースのパーティショニングを実装します。

```sql
-- パーティショニングテーブル定義
CREATE TABLE public.request_logs (
    id UUID NOT NULL,
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
) PARTITION BY RANGE (timestamp);

-- 月次パーティション作成例
CREATE TABLE public.request_logs_y2025m01 PARTITION OF public.request_logs
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
CREATE TABLE public.request_logs_y2025m02 PARTITION OF public.request_logs
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');
-- 以下同様に月次パーティションを作成

-- プライマリキー制約（各パーティションに適用）
ALTER TABLE public.request_logs_y2025m01 ADD PRIMARY KEY (id, timestamp);
ALTER TABLE public.request_logs_y2025m02 ADD PRIMARY KEY (id, timestamp);
```

### 3.2 functions テーブル

利用可能な関数（Language-Title）のメタデータを管理するテーブル

```sql
CREATE TABLE meta.functions (
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

-- インデックス
CREATE INDEX idx_functions_language ON meta.functions (language);
CREATE INDEX idx_functions_language_title ON meta.functions (language_title);
CREATE INDEX idx_functions_is_active ON meta.functions (is_active);
CREATE INDEX idx_functions_tags ON meta.functions USING GIN (tags);
```

### 3.3 runtime_metrics テーブル

ランタイムパフォーマンスメトリクスを保存するテーブル

```sql
CREATE TABLE analytics.runtime_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    language_title VARCHAR(128) NOT NULL,
    avg_duration_ms FLOAT NOT NULL,
    p50_duration_ms FLOAT NOT NULL,
    p90_duration_ms FLOAT NOT NULL,
    p95_duration_ms FLOAT NOT NULL,
    p99_duration_ms FLOAT NOT NULL,
    request_count INTEGER NOT NULL,
    error_count INTEGER NOT NULL,
    cache_hit_count INTEGER NOT NULL,
    memory_usage_mb FLOAT,
    cpu_usage_percent FLOAT
);

-- インデックス
CREATE INDEX idx_runtime_metrics_timestamp ON analytics.runtime_metrics (timestamp);
CREATE INDEX idx_runtime_metrics_language_title ON analytics.runtime_metrics (language_title);
```

### 3.4 error_logs テーブル

エラー詳細を保存する専用テーブル

```sql
CREATE TABLE public.error_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_log_id UUID NOT NULL REFERENCES public.request_logs(id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_code VARCHAR(64) NOT NULL,
    error_message TEXT NOT NULL,
    stack_trace TEXT,
    context JSONB
);

-- インデックス
CREATE INDEX idx_error_logs_request_log_id ON public.error_logs (request_log_id);
CREATE INDEX idx_error_logs_error_code ON public.error_logs (error_code);
CREATE INDEX idx_error_logs_timestamp ON public.error_logs (timestamp);
```

### 3.5 cache_metrics テーブル

キャッシュパフォーマンスメトリクスを保存するテーブル

```sql
CREATE TABLE analytics.cache_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    language_title VARCHAR(128) NOT NULL,
    cache_hits INTEGER NOT NULL,
    cache_misses INTEGER NOT NULL,
    cache_hit_ratio FLOAT NOT NULL,
    avg_cache_latency_ms FLOAT NOT NULL,
    evictions INTEGER NOT NULL,
    memory_usage_mb FLOAT NOT NULL
);

-- インデックス
CREATE INDEX idx_cache_metrics_timestamp ON analytics.cache_metrics (timestamp);
CREATE INDEX idx_cache_metrics_language_title ON analytics.cache_metrics (language_title);
```

### 3.6 users テーブル

APIユーザー情報を管理するテーブル

```sql
CREATE TABLE meta.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(128) UNIQUE NOT NULL,
    name VARCHAR(256),
    email VARCHAR(256),
    api_key_hash VARCHAR(128) UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    rate_limit_per_minute INTEGER NOT NULL DEFAULT 100,
    allowed_languages VARCHAR(32)[],
    role VARCHAR(32) NOT NULL DEFAULT 'user'
);

-- インデックス
CREATE INDEX idx_users_user_id ON meta.users (user_id);
CREATE INDEX idx_users_api_key_hash ON meta.users (api_key_hash);
CREATE INDEX idx_users_is_active ON meta.users (is_active);
```

### 3.7 sessions テーブル

関数実行セッション情報を管理するテーブル（初期化リクエストとパラメータリクエストの2段階処理をサポート）

```sql
CREATE TABLE meta.sessions (
    request_id VARCHAR(64) PRIMARY KEY,
    language_title VARCHAR(128) NOT NULL,
    user_id VARCHAR(128) REFERENCES meta.users(user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_executed_at TIMESTAMPTZ,
    execution_count INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    context JSONB,
    metadata JSONB
);

-- インデックス
CREATE INDEX idx_sessions_language_title ON meta.sessions (language_title);
CREATE INDEX idx_sessions_user_id ON meta.sessions (user_id);
CREATE INDEX idx_sessions_status ON meta.sessions (status);
CREATE INDEX idx_sessions_expires_at ON meta.sessions (expires_at);

-- 有効期限切れセッションクリーンアップ関数
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

-- セッション更新関数
CREATE OR REPLACE FUNCTION meta.update_session_on_execute()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_executed_at = NOW();
    NEW.execution_count = NEW.execution_count + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- セッション更新トリガー
CREATE TRIGGER update_session_on_execute
BEFORE UPDATE ON meta.sessions
FOR EACH ROW
WHEN (NEW.execution_count > OLD.execution_count)
EXECUTE FUNCTION meta.update_session_on_execute();
```

## 4. ビュー定義

### 4.1 daily_usage_stats ビュー

日次使用統計を提供するビュー

```sql
CREATE VIEW analytics.daily_usage_stats AS
SELECT
    DATE_TRUNC('day', timestamp) AS day,
    language_title,
    COUNT(*) AS total_requests,
    COUNT(*) FILTER (WHERE status_code >= 200 AND status_code < 300) AS successful_requests,
    COUNT(*) FILTER (WHERE status_code >= 400) AS error_requests,
    COUNT(*) FILTER (WHERE cached = TRUE) AS cached_requests,
    AVG(duration_ms) AS avg_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    COUNT(DISTINCT user_id) AS unique_users
FROM
    public.request_logs
GROUP BY
    DATE_TRUNC('day', timestamp),
    language_title
ORDER BY
    day DESC,
    total_requests DESC;
```

### 4.2 function_performance ビュー

関数ごとのパフォーマンス統計を提供するビュー

```sql
CREATE VIEW analytics.function_performance AS
SELECT
    language_title,
    COUNT(*) AS total_requests,
    AVG(duration_ms) AS avg_duration_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    COUNT(*) FILTER (WHERE status_code >= 400) AS error_count,
    COUNT(*) FILTER (WHERE cached = TRUE) AS cache_hit_count,
    ROUND((COUNT(*) FILTER (WHERE cached = TRUE))::NUMERIC / COUNT(*)::NUMERIC * 100, 2) AS cache_hit_ratio
FROM
    public.request_logs
WHERE
    timestamp > NOW() - INTERVAL '7 days'
GROUP BY
    language_title
ORDER BY
    total_requests DESC;
```

### 4.3 error_summary ビュー

エラー概要を提供するビュー

```sql
CREATE VIEW analytics.error_summary AS
SELECT
    e.error_code,
    COUNT(*) AS error_count,
    MIN(e.timestamp) AS first_occurrence,
    MAX(e.timestamp) AS last_occurrence,
    r.language_title,
    MODE() WITHIN GROUP (ORDER BY r.status_code) AS most_common_status
FROM
    public.error_logs e
JOIN
    public.request_logs r ON e.request_log_id = r.id
WHERE
    e.timestamp > NOW() - INTERVAL '7 days'
GROUP BY
    e.error_code,
    r.language_title
ORDER BY
    error_count DESC;
```

## 5. 関数とトリガー

### 5.1 更新タイムスタンプ自動化

```sql
-- 更新タイムスタンプ自動化関数
CREATE OR REPLACE FUNCTION meta.update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- functionsテーブルへのトリガー適用
CREATE TRIGGER update_functions_timestamp
BEFORE UPDATE ON meta.functions
FOR EACH ROW
EXECUTE FUNCTION meta.update_timestamp();

-- usersテーブルへのトリガー適用
CREATE TRIGGER update_users_timestamp
BEFORE UPDATE ON meta.users
FOR EACH ROW
EXECUTE FUNCTION meta.update_timestamp();
```

### 5.2 メトリクス集計自動化

```sql
-- 日次メトリクス集計関数
CREATE OR REPLACE FUNCTION analytics.aggregate_daily_metrics()
RETURNS void AS $$
DECLARE
    target_date DATE := CURRENT_DATE - 1; -- 前日
BEGIN
    -- ランタイムメトリクス集計
    INSERT INTO analytics.runtime_metrics (
        timestamp,
        language_title,
        avg_duration_ms,
        p50_duration_ms,
        p90_duration_ms,
        p95_duration_ms,
        p99_duration_ms,
        request_count,
        error_count,
        cache_hit_count,
        memory_usage_mb,
        cpu_usage_percent
    )
    SELECT
        DATE_TRUNC('day', timestamp) AS day,
        language_title,
        AVG(duration_ms) AS avg_duration_ms,
        PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms) AS p50_duration_ms,
        PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY duration_ms) AS p90_duration_ms,
        PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
        PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
        COUNT(*) AS request_count,
        COUNT(*) FILTER (WHERE status_code >= 400) AS error_count,
        COUNT(*) FILTER (WHERE cached = TRUE) AS cache_hit_count,
        AVG((runtime_metrics->>'memory_usage_mb')::FLOAT) AS memory_usage_mb,
        AVG((runtime_metrics->>'cpu_usage_percent')::FLOAT) AS cpu_usage_percent
    FROM
        public.request_logs
    WHERE
        DATE(timestamp) = target_date
    GROUP BY
        DATE_TRUNC('day', timestamp),
        language_title;
        
    -- キャッシュメトリクス集計
    INSERT INTO analytics.cache_metrics (
        timestamp,
        language_title,
        cache_hits,
        cache_misses,
        cache_hit_ratio,
        avg_cache_latency_ms,
        evictions,
        memory_usage_mb
    )
    SELECT
        DATE_TRUNC('day', timestamp) AS day,
        language_title,
        COUNT(*) FILTER (WHERE cached = TRUE) AS cache_hits,
        COUNT(*) FILTER (WHERE cached = FALSE) AS cache_misses,
        CASE 
            WHEN COUNT(*) > 0 THEN 
                ROUND((COUNT(*) FILTER (WHERE cached = TRUE))::NUMERIC / COUNT(*)::NUMERIC, 4)
            ELSE 0 
        END AS cache_hit_ratio,
        AVG(CASE WHEN cached = TRUE THEN duration_ms ELSE NULL END) AS avg_cache_latency_ms,
        0 AS evictions, -- 実際のRedisメトリクスから取得する必要あり
        0 AS memory_usage_mb -- 実際のRedisメトリクスから取得する必要あり
    FROM
        public.request_logs
    WHERE
        DATE(timestamp) = target_date
    GROUP BY
        DATE_TRUNC('day', timestamp),
        language_title;
END;
$$ LANGUAGE plpgsql;
```

## 6. データ保持ポリシー

### 6.1 パーティション管理

古いデータを効率的に管理するためのパーティション削除ポリシー

```sql
-- 古いパーティションを削除する関数
CREATE OR REPLACE FUNCTION public.drop_old_partitions(retention_months INTEGER)
RETURNS void AS $$
DECLARE
    partition_name TEXT;
    partition_date DATE;
    cutoff_date DATE := CURRENT_DATE - (retention_months * 30);
BEGIN
    FOR partition_name, partition_date IN
        SELECT
            relname,
            to_date(substring(relname from 'request_logs_y(\d{4})m(\d{2})'), 'YYYYMM')
        FROM
            pg_class
        WHERE
            relname LIKE 'request_logs_y%m%'
            AND relkind = 'r'
    LOOP
        IF partition_date < cutoff_date THEN
            EXECUTE 'DROP TABLE IF EXISTS public.' || partition_name;
            RAISE NOTICE 'Dropped partition: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;
```

### 6.2 集計データ保持

```sql
-- 古い集計データを削除するポリシー
CREATE OR REPLACE FUNCTION analytics.cleanup_old_metrics(retention_days INTEGER)
RETURNS void AS $$
BEGIN
    -- ランタイムメトリクスのクリーンアップ
    DELETE FROM analytics.runtime_metrics
    WHERE timestamp < CURRENT_DATE - retention_days;
    
    -- キャッシュメトリクスのクリーンアップ
    DELETE FROM analytics.cache_metrics
    WHERE timestamp < CURRENT_DATE - retention_days;
    
    RAISE NOTICE 'Cleaned up metrics older than % days', retention_days;
END;
$$ LANGUAGE plpgsql;
```

## 7. バックアップ戦略

### 7.1 バックアップ設定

```sql
-- バックアップロール作成
CREATE ROLE backup_role WITH LOGIN PASSWORD 'secure_password';

-- バックアップ権限付与
GRANT CONNECT ON DATABASE lambda_logs TO backup_role;
GRANT CONNECT ON DATABASE lambda_meta TO backup_role;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backup_role;
GRANT SELECT ON ALL TABLES IN SCHEMA meta TO backup_role;
GRANT SELECT ON ALL TABLES IN SCHEMA analytics TO backup_role;

-- 自動バックアップ設定（pg_dump使用）
-- 実際のバックアップはシステム側で設定
```

### 7.2 復元手順

```bash
# フルバックアップからの復元
pg_restore -h hostname -U username -d lambda_logs -v backup_file.dump

# 特定テーブルの復元
pg_restore -h hostname -U username -d lambda_logs -t request_logs -v backup_file.dump
```

## 8. セキュリティ設定

### 8.1 ロールとアクセス権限

```sql
-- アプリケーションロール
CREATE ROLE app_role WITH LOGIN PASSWORD 'secure_password';
GRANT CONNECT ON DATABASE lambda_logs, lambda_meta TO app_role;
GRANT USAGE ON SCHEMA public, meta, analytics TO app_role;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO app_role;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA meta TO app_role;
GRANT SELECT ON ALL TABLES IN SCHEMA analytics TO app_role;

-- 読み取り専用ロール（監視・分析用）
CREATE ROLE readonly_role WITH LOGIN PASSWORD 'secure_password';
GRANT CONNECT ON DATABASE lambda_logs, lambda_meta TO readonly_role;
GRANT USAGE ON SCHEMA public, meta, analytics TO readonly_role;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly_role;
GRANT SELECT ON ALL TABLES IN SCHEMA meta TO readonly_role;
GRANT SELECT ON ALL TABLES IN SCHEMA analytics TO readonly_role;

-- 管理者ロール
CREATE ROLE admin_role WITH LOGIN PASSWORD 'secure_password' SUPERUSER;
```

### 8.2 行レベルセキュリティ（RLS）

```sql
-- ユーザーテーブルのRLS
ALTER TABLE meta.users ENABLE ROW LEVEL SECURITY;

-- ポリシー: 自分のデータのみ閲覧可能
CREATE POLICY user_view_own ON meta.users
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', TRUE));

-- ポリシー: 管理者はすべて閲覧可能
CREATE POLICY admin_view_all ON meta.users
    FOR ALL
    USING (current_setting('app.current_role', TRUE) = 'admin');
```

## 9. パフォーマンスチューニング

### 9.1 インデックス戦略

- 頻繁にクエリされるカラムにインデックスを作成
- 複合インデックスを適切に使用
- 大きなテーブルはパーティショニングで分割
- 定期的なインデックス再構築を実施

### 9.2 クエリ最適化

- 頻繁に実行されるクエリはマテリアライズドビューを検討
- 実行計画の定期的な確認
- 不要なJOINやサブクエリの排除

```sql
-- マテリアライズドビュー例
CREATE MATERIALIZED VIEW analytics.daily_stats_mv AS
SELECT * FROM analytics.daily_usage_stats;

-- 定期的な更新
REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.daily_stats_mv;
```

## 10. 移行・バージョン管理

### 10.1 マイグレーションスクリプト

データベースのバージョン管理とマイグレーションには、Flyway、Liquibase、またはカスタムスクリプトを使用します。

```sql
-- バージョン管理テーブル
CREATE TABLE meta.schema_version (
    id SERIAL PRIMARY KEY,
    version VARCHAR(32) NOT NULL,
    description TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied_by VARCHAR(128) NOT NULL
);

-- 初期バージョン登録
INSERT INTO meta.schema_version (version, description, applied_by)
VALUES ('1.0.0', 'Initial schema creation', 'system');
```

### 10.2 ダウンタイムなしの変更戦略

- 新しいカラム追加: `ALTER TABLE ... ADD COLUMN`
- インデックス作成: `CREATE INDEX CONCURRENTLY`
- テーブル再構築: 一時テーブル作成 → データ移行 → 名前変更
