# Rust Controller 技術仕様書

## 1. 概要

Rust Controllerは、lambda-microservice基盤の中核コンポーネントとして、外部からのリクエストを受け取り、適切なランタイムコンテナに処理を委譲する役割を担います。高性能と低レイテンシを実現するためにRust言語で実装されます。セッションベースの実行モデルを採用し、初期化リクエストとパラメータリクエストの2段階処理を行います。

## 2. 責務

- リクエストヘッダ（Language-Title）の解析
- セッション管理（作成・取得・有効期限管理）
- スクリプト本体の動的登録と管理
- 適切なランタイムコンテナの選択
- Redisキャッシュの管理（読み取り/書き込み）
- ランタイムコンテナとの通信
- 実行結果の処理
- PostgreSQLへの処理ログ、セッション情報、スクリプト本体の永続化
- メトリクスの収集と公開

## 3. アーキテクチャ

```
                  ┌─────────────────────────────────────────────────────────┐
                  │                    Rust Controller                      │
┌─────────┐       │                                                         │       ┌─────────────┐
│ OpenFaaS │       │ ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌───────────┐ │       │  Runtime    │
│ Gateway  │──────▶│ │ Request │─▶│ Session  │─▶│ Workflow │─▶│ Runtime   │─│──────▶│ Containers  │
└─────────┘       │ │ Parser  │  │ Manager  │  │ Manager  │  │ Selector  │ │       └─────────────┘
                  │ └─────────┘  └──────────┘  └─────────┘  └───────────┘ │             │
                  │       │            │            │             ▲        │             │
                  └───────┼────────────┼────────────┼─────────────┼────────┘             │
                          │            │            │             │                      │
                          ▼            ▼            ▼             │                      │
                  ┌─────────────┐    ┌─────────┐    ┌─────────┐   │                      │
                  │    Redis    │    │PostgreSQL│◀───┴─────────┴───┴──────────────────────┘
                  └─────────────┘    └─────────┘
```

## 4. コンポーネント詳細

### 4.1 Request Parser

- **機能**: HTTPリクエストを解析し、ヘッダとペイロードを抽出
- **入力**: OpenFaaS Gatewayからのリクエスト
- **出力**: 構造化されたリクエストオブジェクト
- **エラー処理**: 不正なリクエスト形式の検出と適切なエラーレスポンス
- **リクエスト種別判定**: 初期化リクエストかパラメータリクエストかを判定

### 4.2 Session Manager

- **機能**: セッション管理とスクリプト登録を担当
- **責務**:
  - セッション作成（初期化リクエスト時）
  - スクリプト本体の登録と保存
  - セッション取得（パラメータリクエスト時）
  - セッション有効期限管理
  - セッション永続化（PostgreSQL）
  - スクリプト本体の永続化（PostgreSQL）
  - セッションとスクリプトの復元（サーバー再起動時）
  - 定期的なセッションクリーンアップ
  - コンパイル型言語（Rust等）のスクリプトコンパイル管理

### 4.3 Workflow Manager

- **機能**: リクエスト処理の全体フローを制御
- **責務**:
  - Redisキャッシュの確認
  - キャッシュヒット時の即時レスポンス
  - ランタイム選択の委譲
  - 実行結果の処理
  - レスポンス生成
  - メトリクス収集

### 4.4 Runtime Selector

- **機能**: Language-Titleに基づいて適切なランタイムコンテナを選択し、スクリプト実行を管理
- **選択ロジック**:
  - プレフィックスマッチング（例: "nodejs-", "python-", "rust-"）
  - 設定ファイルベースのマッピング
  - 動的ディスカバリー（Kubernetes Service Discovery利用）
- **スクリプト実行管理**:
  - 動的に登録されたスクリプトの実行環境準備
  - インタープリタ言語（Node.js、Python）のスクリプト直接実行
  - コンパイル型言語（Rust）のスクリプトコンパイルと実行
  - WebAssembly変換とキャッシュ（Rust等のコンパイル型言語用）

### 4.5 Cache Manager

- **機能**: Redisとの通信を管理
- **操作**:
  - キャッシュ読み取り
  - キャッシュ書き込み
  - TTL（有効期限）管理
  - キャッシュ無効化
  - セッションデータのキャッシュ

### 4.6 Database Logger

- **機能**: 実行ログとセッション情報をPostgreSQLに永続化
- **記録項目**:
  - リクエストID
  - タイムスタンプ
  - Language-Title
  - リクエストペイロード
  - レスポンスデータ
  - 実行時間
  - ステータスコード
  - セッション情報

### 4.7 Metrics Collector

- **機能**: Prometheusメトリクスの収集と公開
- **メトリクス**:
  - リクエスト数
  - レスポンス時間
  - エラー率
  - キャッシュヒット率
  - ランタイム別実行時間
  - アクティブセッション数
  - セッション作成率

## 5. API仕様

### 5.1 内部API

#### 初期化リクエスト受信エンドポイント

```
POST /initialize
Content-Type: application/json
Language-Title: {language}-{title}

{
  "context": {
    "environment": "production",
    "user_id": "user-123",
    "timeout_ms": 30000,
    "retain_session": true,
    "compile_options": {
      "optimization_level": "release",
      "features": ["feature1", "feature2"],
      "target": "x86_64-unknown-linux-gnu"
    }
  },
  "script_content": "// スクリプト本体\nmodule.exports = async (event) => {\n  // 処理ロジック\n};"
}
```

#### パラメータリクエスト受信エンドポイント

```
POST /execute/{request_id}
Content-Type: application/json

{
  "params": {...}
}
```

#### セッション状態取得エンドポイント

```
GET /sessions/{request_id}
```

#### ランタイムコンテナ通信

```
POST /runtime/{language}/{title}
Content-Type: application/json

{
  "params": {...},
  "context": {...},
  "request_id": "..."
}
```

### 5.2 外部API（Prometheus Metrics）

```
GET /metrics
```

## 6. エラー処理

| エラーケース | 対応 | HTTPステータス |
|------------|------|--------------|
| 不正なLanguage-Title | エラーレスポンス返却 | 400 Bad Request |
| ランタイム不明 | エラーレスポンス返却 | 404 Not Found |
| ランタイム実行エラー | エラー内容をログに記録し返却 | 500 Internal Server Error |
| タイムアウト | タイムアウトエラー返却 | 504 Gateway Timeout |
| Redis接続エラー | フォールバック（キャッシュなし実行） | 処理継続 |
| PostgreSQL接続エラー | 非同期リトライキュー追加 | 処理継続 |

## 7. パフォーマンス要件

- リクエスト処理時間: 平均 < 10ms（コントローラ内部処理のみ）
- メモリ使用量: < 256MB
- CPU使用率: 平均 < 30%
- 同時接続数: 最大10,000
- スレッド数: CPU論理コア数 x 2

## 8. セキュリティ考慮事項

- 入力バリデーション
- レートリミット
- ヘッダインジェクション対策
- 認証トークン検証
- ログ内の機密情報マスキング

## 9. 設定パラメータ

| パラメータ | 説明 | デフォルト値 |
|-----------|------|------------|
| REDIS_URL | Redisサーバー接続URL | redis://redis:6379 |
| POSTGRES_URL | PostgreSQL接続URL | postgres://user:pass@postgres:5432/logs |
| CACHE_TTL | キャッシュ有効期限（秒） | 3600 |
| REQUEST_TIMEOUT | リクエストタイムアウト（ミリ秒） | 5000 |
| LOG_LEVEL | ログレベル | info |
| METRICS_PORT | Prometheusメトリクスポート | 9090 |

## 10. 依存関係

- **フレームワーク**: actix-web / warp / tokio
- **データベース**: tokio-postgres / sqlx
- **キャッシュ**: redis-rs
- **メトリクス**: prometheus-client-rust
- **ロギング**: tracing / log
- **シリアライゼーション**: serde
- **HTTP**: reqwest / hyper

## 11. デプロイメント

Rustコントローラは、以下の形式でデプロイされます：

- Dockerコンテナ
- Kubernetes Pod
- OpenFaaS Function

Dockerfileの例:

```dockerfile
FROM rust:1.67 as builder
WORKDIR /usr/src/controller
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /usr/src/controller/target/release/controller /usr/local/bin/
EXPOSE 8080
EXPOSE 9090
CMD ["controller"]
```
