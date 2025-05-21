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

### クイックスタート（ローカル開発）

ローカル開発環境をすばやくセットアップするには、以下のコマンドを実行してください：

```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
git checkout devin/local-development  # ローカル開発用ブランチに切り替え
chmod +x scripts/*.sh                 # スクリプトに実行権限を付与
./scripts/setup_local_dev.sh          # ローカル開発環境をセットアップ
```

### ローカル開発環境の詳細

ローカル開発ブランチ（`devin/local-development`）には、以下の機能が含まれています：

1. **ルートレベルの`.env`ファイル**：ローカル開発用のデフォルト設定値を含むファイルです。Docker Compose内のサービス名を使用するように設定されています。

2. **サンプルデータ初期化スクリプト**：`init_sample_data.sh`スクリプトは、以下の3つのサンプル関数を作成します：
   - **Node.js計算機**：四則演算を実行する計算機関数
   - **Pythonテキスト処理**：テキストの単語数カウント、文字数カウント、大文字/小文字変換などの機能
   - **Rustデータ検証**：データの検証ルールを適用する機能

3. **セットアップスクリプト**：`setup_local_dev.sh`スクリプトは、以下の処理を自動化します：
   - 前提条件のチェック（Docker、Docker Compose、PostgreSQLクライアント）
   - 環境変数ファイルの作成または確認
   - データベースの起動とマイグレーション
   - サンプルデータの初期化
   - すべてのサービスの起動
   - ランタイムのテスト

セットアップが完了すると、以下のURLでサービスにアクセスできます：
- Controller: http://localhost:8080
- Node.js Runtime: http://localhost:8081
- Python Runtime: http://localhost:8082
- Rust Runtime: http://localhost:8083

### 手動セットアップ

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

詳細なAPI仕様は[API仕様書](./docs/api/api_specification.md)を参照してください。主要なエンドポイントの概要は以下の通りです。

### 認証・認可

APIはJWTベースの認証を使用します。

```
Authorization: Bearer {token}
```

### 初期化 API

```
POST /api/v1/initialize
Header: Authorization: Bearer {token}
Header: Language-Title: <language>-<title>
Body: {
  "context": { ... },
  "script_content": "..."
}
```

### 実行 API

```
POST /api/v1/execute/{request_id}
Header: Authorization: Bearer {token}
Body: {
  "params": { ... }
}
```

### セッション状態取得 API

```
GET /api/v1/sessions/{request_id}
Header: Authorization: Bearer {token}
```

### その他のAPI

- スクリプト一覧取得: `GET /api/v1/functions`
- スクリプト詳細取得: `GET /api/v1/functions/{language_title}`
- ヘルスチェック: `GET /health`
- メトリクス: `GET /metrics`

### JSONスキーマとエラーコード

リクエスト/レスポンスのJSONスキーマや、エラーコードの詳細については[API仕様書](./docs/api/api_specification.md)を参照してください。

## 技術仕様

コントローラの内部設計や内部APIの詳細については[Rust Controller 技術仕様書](./docs/technical/rust_controller_spec.md)を参照してください。

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
