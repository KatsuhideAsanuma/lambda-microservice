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
- **自動セッション管理**: セッションテーブルが自動作成され、リクエストごとにセッションが管理されます。
- **完全自動化された起動**: 毎回クリーンな状態でマイグレーションとサンプルデータ投入が自動実行されます。
- **マイクロサービス志向**: 永続化ボリュームを削除し、ステートレスな真のマイクロサービスとして動作します。

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

現在の実装では完全自動化されているため、手動セットアップは非常にシンプルです：

1. リポジトリをクローン:

```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
```

2. すべてのサービスを起動（マイグレーションとサンプルデータ投入が自動実行されます）:

```bash
docker-compose up -d
```

3. 動作確認:

```bash
curl http://localhost:8080/health
```

**注意**: 永続化ボリュームを削除しているため、毎回クリーンな状態で起動し、マイグレーションとサンプルデータ投入が自動実行されます。これにより、真のマイクロサービスとして動作します。

## API エンドポイント

詳細なAPI仕様は[API仕様書](./docs/api/api_specification.md)を参照してください。この仕様書には以下の情報が含まれています：

- 認証・認可の詳細（JWTトークン形式、スコープ、有効期限）
- 各エンドポイントの完全なリクエスト/レスポンスの例
- すべてのエラーコードと対応するメッセージ
- レート制限とクォータ情報
- APIバージョニング方針

### 認証・認可

APIはJWTベースの認証を使用します。詳細な認証フローと必要な権限については[認証・認可仕様](./docs/api/api_specification.md#認証認可)を参照してください。

```
Authorization: Bearer {token}
```

### 初期化 API

セッションを初期化し、実行環境を準備します。詳細な仕様と例については[初期化API仕様](./docs/api/api_specification.md#初期化api)を参照してください。

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

初期化済みのセッションでコードを実行します。詳細な仕様と例については[実行API仕様](./docs/api/api_specification.md#実行api)を参照してください。

```
POST /api/v1/execute/{request_id}
Header: Authorization: Bearer {token}
Body: {
  "params": { ... }
}
```

### セッション状態取得 API

セッションの現在の状態を取得します。詳細な仕様と例については[セッション状態取得API仕様](./docs/api/api_specification.md#セッション状態取得api)を参照してください。

```
GET /api/v1/sessions/{request_id}
Header: Authorization: Bearer {token}
```

### その他のAPI

以下のAPIエンドポイントも利用可能です。詳細な仕様と例については[その他のAPI仕様](./docs/api/api_specification.md#その他のapi)を参照してください。

- スクリプト一覧取得: `GET /api/v1/functions`
- スクリプト詳細取得: `GET /api/v1/functions/{language_title}`
- ヘルスチェック: `GET /health`
- メトリクス: `GET /metrics`

### JSONスキーマとエラーコード

すべてのリクエスト/レスポンスのJSONスキーマや、エラーコードの詳細については[JSONスキーマとエラーコード](./docs/api/api_specification.md#jsonスキーマとエラーコード)を参照してください。エラーコードには以下のカテゴリが含まれます：

- 認証エラー（401, 403）
- 入力検証エラー（400）
- リソース不足エラー（429, 507）
- 内部サーバーエラー（500, 503）
- ランタイム固有のエラー（460-499）

## 技術仕様

コントローラの内部設計や内部APIの詳細については[Rust Controller 技術仕様書](./docs/technical/rust_controller_spec.md)を参照してください。

## データベーススキーマ

主要なテーブル:

- **request_logs**: リクエスト実行ログ
- **error_logs**: エラーログ
- **functions**: 関数メタデータ
- **scripts**: スクリプト本体
- **sessions**: セッション管理（新規追加）

## 開発ガイド

### 新しいランタイムの追加

1. ランタイムコンテナの Dockerfile を作成
2. docker-compose.yml に新しいサービスを追加
3. コントローラの設定を更新

### データベースマイグレーション

現在の実装では、データベースマイグレーションは完全自動化されています：

**自動実行されるマイグレーション:**
- PostgreSQLコンテナ起動時に`database/migrations/`のすべてのマイグレーションファイルが自動実行されます
- バージョン管理されたマイグレーションスクリプト（V1.0.0〜V1.0.6）が含まれています
- セッションテーブルとサンプルデータが自動作成されます

**新しいマイグレーションの追加（開発者向け）:**

```bash
# 新しいマイグレーションファイルを作成
touch database/migrations/V<version>__<description>.sql

# 次回のコンテナ起動時に自動実行されます
docker-compose restart postgres
```

**含まれるマイグレーション:**
- V1.0.0: 初期スキーマ
- V1.0.1: スキーマ作成
- V1.0.2: リクエストログテーブル
- V1.0.3: 関数・スクリプトテーブル
- V1.0.4: エラーログテーブル
- V1.0.5: セッションテーブル（新規追加）
- V1.0.6: デフォルト関数投入（新規追加）

## 設計ドキュメント

詳細な設計ドキュメントは [docs/](./docs/) ディレクトリにあります。
