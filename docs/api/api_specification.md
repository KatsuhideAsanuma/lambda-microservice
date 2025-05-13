# API 仕様書

## 1. 概要

lambda-microservice基盤のAPI仕様を定義します。このAPIは、外部クライアントからのリクエストを受け付け、指定された言語ランタイムでスクリプトを実行し、結果を返却します。

## 2. エンドポイント

### 2.1 初期化リクエスト API

```
POST /api/v1/initialize
Host: api.lambda-microservice.example.com
```

このエンドポイントは、関数実行のセッションを初期化し、リクエストIDを返します。

#### リクエストヘッダ

| ヘッダ名 | 必須 | 説明 |
|---------|------|------|
| Content-Type | 必須 | application/json |
| Authorization | 必須 | Bearer {token} |
| Language-Title | 必須 | 実行言語とスクリプトタイトル（例: nodejs-calculator） |
| X-Request-ID | オプション | クライアント側で生成したリクエストID |

#### リクエスト本文

```json
{
  "context": {
    // 実行コンテキスト情報
    "environment": "production",
    "user_id": "user-123",
    "timeout_ms": 30000,  // ミリ秒単位のタイムアウト（デフォルト: 30000）
    "retain_session": true  // セッションを保持するかどうか
  }
}
```

#### レスポンス

**成功時 (200 OK)**

```json
{
  "request_id": "f8c3de3d-1234-5678-9abc-def012345678",
  "status": "initialized",
  "expires_at": "2025-05-13T12:28:22Z"
}
```

**エラー時**

```json
{
  "error": {
    "code": "INITIALIZATION_ERROR",
    "message": "エラーメッセージ",
    "details": {
      // エラー詳細情報
    }
  }
}
```

### 2.2 パラメータ実行 API

```
POST /api/v1/execute/{request_id}
Host: api.lambda-microservice.example.com
```

このエンドポイントは、初期化で取得したリクエストIDを使用して、パラメータを送信し関数を実行します。

#### リクエストヘッダ

| ヘッダ名 | 必須 | 説明 |
|---------|------|------|
| Content-Type | 必須 | application/json |
| Authorization | 必須 | Bearer {token} |
| X-Cache-Control | オプション | no-cache（キャッシュを無視）、max-age={seconds}（キャッシュTTL指定） |

#### リクエスト本文

```json
{
  "params": {
    // スクリプト実行に必要なパラメータ
    // 例: 計算機能の場合
    "operation": "add",
    "values": [1, 2, 3]
  }
}
```

#### レスポンス

**成功時 (200 OK)**

```json
{
  "request_id": "f8c3de3d-1234-5678-9abc-def012345678",
  "language_title": "nodejs-calculator",
  "execution_time": 42,  // ミリ秒
  "cached": false,       // キャッシュからの結果かどうか
  "result": {
    // スクリプト実行結果
    // 例: 計算機能の場合
    "value": 6
  }
}
```

**エラー時**

```json
{
  "request_id": "f8c3de3d-1234-5678-9abc-def012345678",
  "error": {
    "code": "RUNTIME_ERROR",
    "message": "エラーメッセージ",
    "details": {
      // エラー詳細情報（言語ランタイムによって異なる）
      "line": 10,
      "column": 5,
      "stack_trace": "..."
    }
  }
}
```

#### ステータスコード

| コード | 説明 |
|--------|------|
| 200 | 成功 |
| 400 | 不正なリクエスト（パラメータ不足、形式不正など） |
| 401 | 認証エラー |
| 403 | 権限エラー |
| 404 | 指定されたリクエストIDが見つからない、または有効期限切れ |
| 429 | レートリミット超過 |
| 500 | サーバー内部エラー |
| 504 | タイムアウト |

### 2.3 セッション状態取得 API

```
GET /api/v1/sessions/{request_id}
Host: api.lambda-microservice.example.com
```

このエンドポイントは、特定のセッション（リクエストID）の状態を取得します。

#### リクエストヘッダ

| ヘッダ名 | 必須 | 説明 |
|---------|------|------|
| Authorization | 必須 | Bearer {token} |

#### レスポンス

```json
{
  "request_id": "f8c3de3d-1234-5678-9abc-def012345678",
  "language_title": "nodejs-calculator",
  "status": "active",
  "created_at": "2025-05-13T10:28:22Z",
  "expires_at": "2025-05-13T12:28:22Z",
  "execution_count": 3,
  "last_executed_at": "2025-05-13T11:15:30Z"
}
```

### 2.2 スクリプト一覧取得 API

```
GET /api/v1/functions
Host: api.lambda-microservice.example.com
```

利用可能なスクリプト（Language-Title）の一覧を取得します。

#### リクエストヘッダ

| ヘッダ名 | 必須 | 説明 |
|---------|------|------|
| Authorization | 必須 | Bearer {token} |

#### クエリパラメータ

| パラメータ | 必須 | 説明 |
|-----------|------|------|
| language | オプション | 特定言語のみフィルタ（例: nodejs） |
| page | オプション | ページ番号（デフォルト: 1） |
| per_page | オプション | 1ページあたりの件数（デフォルト: 20、最大: 100） |

#### レスポンス

```json
{
  "total": 42,
  "page": 1,
  "per_page": 20,
  "functions": [
    {
      "language": "nodejs",
      "title": "calculator",
      "language_title": "nodejs-calculator",
      "description": "基本的な計算機能を提供",
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": "2025-01-02T00:00:00Z"
    },
    {
      "language": "python",
      "title": "image-processor",
      "language_title": "python-image-processor",
      "description": "画像処理機能を提供",
      "created_at": "2025-01-03T00:00:00Z",
      "updated_at": "2025-01-04T00:00:00Z"
    }
    // ...
  ]
}
```

### 2.3 スクリプト詳細取得 API

```
GET /api/v1/functions/{language_title}
Host: api.lambda-microservice.example.com
```

特定のスクリプト（Language-Title）の詳細情報を取得します。

#### リクエストヘッダ

| ヘッダ名 | 必須 | 説明 |
|---------|------|------|
| Authorization | 必須 | Bearer {token} |

#### レスポンス

```json
{
  "language": "nodejs",
  "title": "calculator",
  "language_title": "nodejs-calculator",
  "description": "基本的な計算機能を提供",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-02T00:00:00Z",
  "schema": {
    "params": {
      "type": "object",
      "properties": {
        "operation": {
          "type": "string",
          "enum": ["add", "subtract", "multiply", "divide"],
          "description": "実行する演算"
        },
        "values": {
          "type": "array",
          "items": {
            "type": "number"
          },
          "description": "演算対象の数値配列"
        }
      },
      "required": ["operation", "values"]
    },
    "result": {
      "type": "object",
      "properties": {
        "value": {
          "type": "number",
          "description": "計算結果"
        }
      }
    }
  },
  "examples": [
    {
      "description": "足し算の例",
      "request": {
        "params": {
          "operation": "add",
          "values": [1, 2, 3]
        }
      },
      "response": {
        "result": {
          "value": 6
        }
      }
    }
  ]
}
```

### 2.4 ヘルスチェック API

```
GET /health
Host: api.lambda-microservice.example.com
```

システムの稼働状態を確認します。認証は不要です。

#### レスポンス

```json
{
  "status": "ok",
  "version": "1.0.0",
  "components": {
    "controller": "ok",
    "redis": "ok",
    "postgres": "ok",
    "runtimes": {
      "nodejs": "ok",
      "python": "ok",
      "rust": "ok"
    }
  }
}
```

### 2.5 メトリクス API

```
GET /metrics
Host: api.lambda-microservice.example.com
```

Prometheusフォーマットのメトリクスを提供します。内部監視システム用です。

## 3. 認証・認可

### 3.1 認証方式

APIは、JWTベースの認証を使用します。

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

トークンには以下の情報が含まれます：

- `sub`: ユーザーID
- `exp`: 有効期限
- `scope`: 権限スコープ

### 3.2 権限スコープ

| スコープ | 説明 |
|---------|------|
| `execute:all` | すべてのスクリプト実行権限 |
| `execute:nodejs` | Node.jsスクリプト実行権限 |
| `execute:python` | Pythonスクリプト実行権限 |
| `execute:rust` | Rustスクリプト実行権限 |
| `read:functions` | スクリプト一覧・詳細取得権限 |

## 4. レート制限

APIには以下のレート制限が適用されます：

| エンドポイント | 制限 |
|--------------|------|
| `/api/v1/execute` | 認証済み: 100リクエスト/分、未認証: 10リクエスト/分 |
| `/api/v1/functions` | 認証済み: 60リクエスト/分 |
| `/health` | 制限なし |

レート制限超過時は、429 Too Many Requestsが返却されます。

## 5. エラーコード一覧

| エラーコード | 説明 |
|------------|------|
| `INVALID_REQUEST` | リクエスト形式が不正 |
| `INVALID_PARAMS` | パラメータが不正 |
| `UNAUTHORIZED` | 認証エラー |
| `FORBIDDEN` | 権限エラー |
| `NOT_FOUND` | リソースが見つからない |
| `RATE_LIMITED` | レート制限超過 |
| `RUNTIME_ERROR` | スクリプト実行時エラー |
| `TIMEOUT` | 実行タイムアウト |
| `INTERNAL_ERROR` | サーバー内部エラー |

## 6. バージョニング

APIはセマンティックバージョニングに従います。URLパスにメジャーバージョンを含めます（例: `/api/v1/`）。

- メジャーバージョン: 互換性のない変更
- マイナーバージョン: 後方互換性のある機能追加
- パッチバージョン: 後方互換性のあるバグ修正

## 7. クロスオリジンリソース共有（CORS）

APIは、以下のCORS設定を持ちます：

- `Access-Control-Allow-Origin`: 設定された許可オリジン
- `Access-Control-Allow-Methods`: GET, POST, OPTIONS
- `Access-Control-Allow-Headers`: Content-Type, Authorization, Language-Title, X-Request-ID, X-Cache-Control
- `Access-Control-Max-Age`: 86400（24時間）

## 8. キャッシュ制御

実行結果は、デフォルトで1時間キャッシュされます。クライアントは、`X-Cache-Control`ヘッダを使用してキャッシュ動作を制御できます：

- `X-Cache-Control: no-cache` - キャッシュを使用せず、常に新しい実行結果を取得
- `X-Cache-Control: max-age=3600` - キャッシュTTLを3600秒（1時間）に設定
