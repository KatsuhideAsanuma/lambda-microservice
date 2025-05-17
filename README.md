# Lambda Microservice

高速ラムダマイクロサービス基盤の実装

## 概要

このプロジェクトは、複数のプログラミング言語（Node.js、Python、Rust）でコードを実行できるマイクロサービス基盤を提供します。

### 主要コンポーネント

- **Rustコントローラ**: リクエストを処理し、適切なランタイムにルーティングします。Language-Titleヘッダに基づいて動的にランタイムを選択します。
- **実行コンテナ**: Node.js、Python、Rustの各言語用のランタイム環境を提供します。
- **データストア**: Redisをキャッシュとして使用し、PostgreSQLでログを永続化します。
- **監視**: Prometheus、Grafana、Elastic Stackを使用して監視とロギングを行います。

## 新機能

- **データベースログ永続化**: 実行リクエストとエラーをPostgreSQLに自動的に記録します。
- **拡張エラーハンドリング**: すべてのランタイムで一貫したエラーハンドリングを実装しています。
- **最適化されたコンテナ間通信**: 効率的なHTTP通信とJSON形式の標準化を行いました。
- **データベースマイグレーション**: バージョン管理されたマイグレーションスクリプトを提供します。

## セットアップ方法

### 前提条件

- Docker と Docker Compose
- PostgreSQL クライアント（psql）
- curl, jq（テスト用）

### インストール手順

1. リポジトリをクローン:

```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
```

2. データベースを起動してマイグレーションを実行:

```bash
docker-compose up -d postgres
./scripts/migrate_database.sh
```

3. サンプルデータを初期化:

```bash
./scripts/init_sample_data.sh
```

4. サービスを起動:

```bash
docker-compose up -d
```

5. ランタイムをテスト:

```bash
./scripts/test_runtimes.sh
```

## API エンドポイント

### 初期化 API

```
POST /api/v1/initialize
Header: Language-Title: <language>-<title>
Body: {
  "context": { ... },
  "script_content": "..."
}
```

### 実行 API

```
POST /api/v1/execute/{request_id}
Body: {
  "params": { ... }
}
```

## データベーススキーマ

主要なテーブル:

- **request_logs**: リクエスト実行ログ
- **error_logs**: エラーログ
- **functions**: 関数メタデータ
- **scripts**: スクリプト本体

## 開発ガイド

### 新しいランタイムの追加

1. ランタイムコンテナの Dockerfile を作成
2. docker-compose.yml に新しいサービスを追加
3. コントローラの設定を更新

### データベースマイグレーション

新しいマイグレーションスクリプトを作成:

```bash
touch database/migrations/V<version>__<description>.sql
```

マイグレーションを実行:

```bash
./scripts/migrate_database.sh
```

## 設計ドキュメント

詳細な設計ドキュメントは [docs/](./docs/) ディレクトリにあります。
