# Lambda Microservice 起動時の問題点と解決策

## 作成日時: 2025 年 1 月 15 日 01:35

## 概要

LambdaCalculationNode の統合テスト実行時に、lambda-microservice の起動で複数の問題が発生しました。
本文書では、発生した問題とその解決策を記録し、今後の起動手順を明確化します。

## 発生した主要問題

### 1. Rust ランタイムのビルドエラー（最重要）

**問題**: Rust ランタイムのビルド時に`edition2024`互換性エラーが発生

```
error: feature `edition2024` is required
The package requires the Cargo feature called `edition2024`, but that feature is not stabilized in this version of Cargo (1.83.0).
```

**原因**:

- 依存パッケージ`base64ct-1.8.0`が Rust 1.85 以上を要求
- 現在の Docker イメージでは`rust:1.83-slim`を使用

**対策**:

1. **短期解決策**: Rust ランタイムを除外して起動

   ```bash
   docker-compose up -d postgres redis nodejs-runtime python-runtime controller
   ```

2. **長期解決策**: Dockerfile のベースイメージを更新
   ```dockerfile
   FROM rust:1.85-slim as builder  # 1.83 → 1.85
   ```

### 2. コントローラーのデータベース接続エラー

**問題**: コントローラー起動時にデータベース接続失敗

```
Failed to create database connection pool: Database("Error occurred while creating a new object: db error: FATAL: database \"lambda_microservice\n\" does not exist")
```

**原因**: secret ファイル`db_url.txt`に改行文字が含まれていた

**解決策**: secret ファイルから改行文字を除去

```bash
echo -n "postgres://postgres:postgres@postgres:5432/lambda_microservice" > secrets/db_url.txt
```

### 3. データベーステーブルの未作成

**問題**: 必要なテーブル（`meta.functions`, `meta.scripts`等）が存在しない

```
postgres=# \dt
            List of relations
 Schema |     Name     | Type  |  Owner
--------+--------------+-------+----------
 public | error_logs   | table | postgres
 public | request_logs | table | postgres
```

**原因**: データベースマイグレーションが実行されていない

**解決策**: 手動でマイグレーションを実行

```bash
# スキーマ作成
docker-compose exec postgres psql -U postgres -d lambda_microservice -c "CREATE SCHEMA IF NOT EXISTS meta;"

# テーブル作成
docker-compose exec postgres psql -U postgres -d lambda_microservice -f database/migrations/V1.0.3__create_functions.sql
```

### 4. docker-compose 設定の問題

**問題**: `docker-compose-minimal.yml`では必要なサービスが不足

**差異**:

- minimal 版: PostgreSQL, Redis, Node.js/Python runtime, コントローラーのみ
- 完全版: 上記 + Rust ランタイム, OpenFaaS, Envoy 等

**推奨**: 完全版を使用（Rust ランタイム問題解決後）

### 5. API エンドポイントの利用不可

**問題**: `/api/v1/initialize`および`/api/v1/functions`が利用できない

```
curl http://localhost:8080/api/v1/functions
# => "Requested application data is not configured correctly"
```

**原因**:

1. データベーステーブルの未作成
2. 初期データの未投入
3. コントローラーの不完全な起動

## 推奨起動手順

### Step 1: 環境確認

```bash
cd src/micro_services/lambda-microservice
ls secrets/  # db_url.txt, redis_url.txt, redis_cache_url.txt の存在確認
```

### Step 2: secret ファイルの修正

```bash
# 改行文字を除去
echo -n "postgres://postgres:postgres@postgres:5432/lambda_microservice" > secrets/db_url.txt
echo -n "redis://redis:6379" > secrets/redis_url.txt
echo -n "redis://redis:6379" > secrets/redis_cache_url.txt
```

### Step 3: サービス起動（Rust ランタイム除く）

```bash
docker-compose up -d postgres redis nodejs-runtime python-runtime controller
```

### Step 4: データベース初期化

```bash
# ヘルスチェック
docker-compose exec postgres pg_isready -U postgres

# スキーマ作成
docker-compose exec postgres psql -U postgres -d lambda_microservice -c "CREATE SCHEMA IF NOT EXISTS meta;"

# マイグレーション実行
for migration in database/migrations/*.sql; do
    docker-compose exec postgres psql -U postgres -d lambda_microservice -f "/docker-entrypoint-initdb.d/$(basename $migration)"
done
```

### Step 5: 初期データ投入

```bash
# サンプルデータスクリプト実行
bash scripts/init_sample_data.sh
```

### Step 6: 動作確認

```bash
# ヘルスチェック
curl http://localhost:8080/health

# 関数一覧取得
curl http://localhost:8080/api/v1/functions

# 初期化API確認
curl -X POST http://localhost:8080/api/v1/initialize
```

## 今後の改善項目

### 高優先度

1. **Rust ランタイムの依存関係更新**

   - Cargo.toml の依存パッケージバージョンの互換性確認
   - Dockerfile の Rust バージョン更新

2. **自動マイグレーションの実装**
   - 起動時の自動データベース初期化
   - ヘルスチェック機能の強化

### 中優先度

3. **docker-compose 設定の統一**

   - minimal 版の改善または削除
   - 開発環境向け設定の明確化

4. **秘匿情報管理の改善**
   - secret ファイルのフォーマット検証
   - 環境変数による設定オプションの追加

### 低優先度

5. **ドキュメント整備**
   - 起動手順の自動化スクリプト作成
   - トラブルシューティングガイドの充実

## 関連ファイル

- `docker-compose.yml` - メイン設定ファイル
- `docker-compose-minimal.yml` - 簡易版設定ファイル
- `database/migrations/` - データベースマイグレーションファイル
- `secrets/` - 秘匿情報ファイル
- `scripts/migrate_database.sh` - マイグレーション実行スクリプト
- `scripts/init_sample_data.sh` - 初期データ投入スクリプト

## 作成者

Cline AI Assistant

## 更新履歴

- 2025-01-15 01:35: 初版作成（LambdaCalculationNode 統合テスト対応時の問題記録）
