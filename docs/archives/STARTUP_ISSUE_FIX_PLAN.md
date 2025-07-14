# Lambda Microservice スタートアップ問題修正計画書

## 作成日時: 2025年7月15日
## 対象バージョン: v1.0.x
## 修正目的: データベース初期化の自動化とスタートアップ問題の根本解決

---

## 修正概要

### 対象問題
1. セッションテーブル未作成によるAPI障害
2. 手動マイグレーション実行の必要性
3. 設定ファイルの改行文字問題
4. 初期データの未投入

### 修正方針
- **最小限の変更**: 既存コードの動作を保持しつつ、必要な機能のみ追加
- **段階的実行**: 各修正を独立して実行可能に設計
- **ガード機能**: 既存データの保護と重複実行防止

---

## 修正計画

### Phase 1: データベース初期化の自動化

#### 1.1 マイグレーションファイルの追加
**ファイル**: `database/migrations/V1.0.5__create_sessions.sql`

```sql
-- セッションテーブルの作成（IF NOT EXISTS で安全な実行）
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

-- インデックスの作成（IF NOT EXISTS で重複実行防止）
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
```

**影響範囲**: 新規テーブル作成のみ、既存データへの影響なし
**ガード**: `IF NOT EXISTS`による重複実行防止

#### 1.2 Docker-compose.ymlの修正
**ファイル**: `docker-compose.yml`

```yaml
# PostgreSQLサービスの修正
postgres:
  image: postgres:14-alpine
  environment:
    POSTGRES_USER: postgres
    POSTGRES_PASSWORD: postgres
    POSTGRES_DB: lambda_microservice
  ports:
    - "5432:5432"
  volumes:
    - postgres_data:/var/lib/postgresql/data
    # 初期化スクリプトの追加
    - ./database/migrations:/docker-entrypoint-initdb.d:ro
  healthcheck:
    test: ["CMD-SHELL", "pg_isready -U postgres"]
    interval: 5s
    timeout: 5s
    retries: 5
```

**影響範囲**: PostgreSQLコンテナの初期化処理のみ
**ガード**: 
- 既存データボリュームの保持
- 初期化は空のデータベースでのみ実行

### Phase 2: アプリケーションレベルのガード実装

#### 2.1 データベース整合性チェックの追加
**ファイル**: `controller/src/database.rs`

```rust
impl PostgresPool {
    // 既存メソッドの後に追加
    pub async fn verify_schema(&self) -> Result<()> {
        let required_tables = vec![
            ("meta", "functions"),
            ("meta", "scripts"),
            ("meta", "sessions"),  // 新しく追加
            ("public", "request_logs"),
            ("public", "error_logs"),
        ];
        
        for (schema, table) in required_tables {
            let query = format!(
                "SELECT 1 FROM information_schema.tables 
                 WHERE table_schema = $1 AND table_name = $2"
            );
            
            let result = self.query_opt(&query, &[&schema, &table]).await?;
            if result.is_none() {
                return Err(Error::Database(format!(
                    "Required table {}.{} is missing", schema, table
                )));
            }
        }
        
        Ok(())
    }
}
```

**影響範囲**: 新規メソッド追加のみ
**ガード**: 読み取り専用の検証処理

#### 2.2 main.rsでの起動時検証
**ファイル**: `controller/src/main.rs`

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 既存の設定読み込み処理...
    
    let postgres_pool = PostgresPool::new(&config.database_url)
        .await
        .expect("Failed to create database connection pool");
    
    // 新規追加: スキーマ検証
    postgres_pool.verify_schema()
        .await
        .expect("Database schema verification failed");
    
    info!("Database schema verification completed");
    
    // 既存の処理を継続...
}
```

**影響範囲**: 起動時検証の追加のみ
**ガード**: 検証失敗時の明確なエラーメッセージ

### Phase 3: 設定ファイル問題の解決

#### 3.1 設定読み込み処理の改善
**ファイル**: `controller/src/config.rs`

```rust
impl Config {
    // 既存メソッドの修正
    fn read_secret_file(path: &str) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
            .map(|content| content.trim().to_string()) // 改行文字除去
    }
    
    // 新規追加: 設定検証
    pub fn validate(&self) -> Result<(), String> {
        // URL形式の検証
        if !self.database_url.starts_with("postgres://") {
            return Err("Invalid database URL format".to_string());
        }
        
        if !self.redis_url.starts_with("redis://") {
            return Err("Invalid Redis URL format".to_string());
        }
        
        Ok(())
    }
}
```

**影響範囲**: 設定読み込み処理の改善のみ
**ガード**: 設定値の検証と明確なエラーメッセージ

### Phase 4: 初期データの投入

#### 4.1 デフォルト関数の定義
**ファイル**: `database/migrations/V1.0.6__insert_default_functions.sql`

```sql
-- デフォルト関数の投入（重複実行防止）
INSERT INTO meta.functions (
    id, language, title, language_title, description, 
    schema_definition, examples, created_at, updated_at, 
    created_by, is_active, version, tags
) VALUES 
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d479'::uuid,
    'nodejs', 'calculator', 'nodejs-calculator',
    'Basic calculator functions in Node.js',
    '{"type": "object", "properties": {"operation": {"type": "string"}, "a": {"type": "number"}, "b": {"type": "number"}}}',
    '[{"operation": "add", "a": 5, "b": 3, "result": 8}]',
    NOW(), NOW(), NULL, true, '1.0.0', 
    ARRAY['math', 'basic']
)
ON CONFLICT (language_title) DO NOTHING;

-- 他の言語版も同様に追加...
```

**影響範囲**: 新規データ投入のみ
**ガード**: `ON CONFLICT DO NOTHING`による重複防止

---

## 実行計画

### 前提条件
- 現在のサービスを停止
- データベースのバックアップ作成
- 設定ファイルの確認

### 実行手順

#### Step 1: バックアップの作成
```bash
# PostgreSQLデータのバックアップ
docker-compose exec postgres pg_dump -U postgres lambda_microservice > backup_$(date +%Y%m%d_%H%M%S).sql

# 設定ファイルのバックアップ
cp -r secrets secrets_backup_$(date +%Y%m%d_%H%M%S)
```

#### Step 2: マイグレーションファイルの追加
```bash
# セッションテーブル作成
cat > database/migrations/V1.0.5__create_sessions.sql << 'EOF'
[上記のSQLを配置]
EOF

# デフォルト関数投入
cat > database/migrations/V1.0.6__insert_default_functions.sql << 'EOF'
[上記のSQLを配置]
EOF
```

#### Step 3: Docker-compose設定の更新
```bash
# docker-compose.ymlの修正
# PostgreSQLサービスにマイグレーションマウントを追加
```

#### Step 4: 設定ファイルの修正
```bash
# 改行文字の除去
echo -n "postgres://postgres:postgres@postgres:5432/lambda_microservice" > secrets/db_url.txt
echo -n "redis://redis:6379" > secrets/redis_url.txt
echo -n "redis://redis:6379" > secrets/redis_cache_url.txt
```

#### Step 5: アプリケーションコードの更新
```bash
# database.rs, main.rs, config.rsの修正
# 段階的にコミット
```

#### Step 6: 検証と起動
```bash
# サービスの起動
docker-compose up -d postgres redis

# マイグレーション実行確認
docker-compose exec postgres psql -U postgres -d lambda_microservice -c "\dt meta.*"

# アプリケーションの起動
docker-compose up -d controller nodejs-runtime python-runtime

# 動作確認
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/functions
```

---

## ガード機能

### 1. データ保護ガード
- **既存データの保護**: `IF NOT EXISTS`による重複実行防止
- **バックアップ機能**: 自動バックアップスクリプトの実行
- **段階的実行**: 各Phaseを独立して実行可能

### 2. 整合性ガード
- **スキーマ検証**: 起動時の必須テーブル確認
- **設定検証**: URL形式と接続可能性の確認
- **依存関係チェック**: サービス間の起動順序確認

### 3. 運用ガード
- **ログ出力**: 各段階での詳細なログ記録
- **エラーハンドリング**: 明確なエラーメッセージと復旧手順
- **ロールバック**: 問題発生時の戻し手順

---

## 成功基準

### 機能テスト
- [ ] `/health` エンドポイントの200レスポンス
- [ ] `/api/v1/functions` エンドポイントの正常動作
- [ ] `/api/v1/initialize` エンドポイントの正常動作
- [ ] セッション作成と取得の正常動作

### 性能テスト
- [ ] 起動時間の劣化なし（現在の±10%以内）
- [ ] メモリ使用量の増加なし
- [ ] API応答時間の劣化なし

### 安定性テスト
- [ ] 再起動時の正常動作
- [ ] データベース再作成時の正常動作
- [ ] 設定変更時の正常動作

---

## ロールバック計画

### 緊急時のロールバック
```bash
# 1. サービス停止
docker-compose down

# 2. 設定ファイル復元
rm -rf secrets && mv secrets_backup_* secrets

# 3. データベース復元
docker-compose up -d postgres
cat backup_*.sql | docker-compose exec -T postgres psql -U postgres lambda_microservice

# 4. 旧バージョンのコード復元
git checkout [前のcommit-hash]

# 5. サービス再起動
docker-compose up -d
```

### 段階的なロールバック
- Phase 4のみ削除: デフォルト関数の削除
- Phase 3のみ削除: 設定検証の無効化
- Phase 2のみ削除: 起動時検証の無効化
- Phase 1のみ削除: 新規テーブルの削除

---

## 実行タイムライン

| Phase | 作業内容 | 予想時間 | 担当者 |
|-------|----------|----------|--------|
| 準備 | バックアップ作成 | 10分 | 運用担当 |
| Phase 1 | マイグレーション追加 | 20分 | 開発担当 |
| Phase 2 | 検証機能追加 | 30分 | 開発担当 |
| Phase 3 | 設定改善 | 15分 | 開発担当 |
| Phase 4 | 初期データ投入 | 10分 | 開発担当 |
| 検証 | 動作確認 | 20分 | 運用担当 |
| **合計** | | **105分** | |

---

## 承認とレビュー

- [ ] 技術レビュー完了
- [ ] セキュリティレビュー完了
- [ ] 運用レビュー完了
- [ ] バックアップ計画確認
- [ ] ロールバック計画確認

**承認者**: _______________
**実行予定日**: _______________
