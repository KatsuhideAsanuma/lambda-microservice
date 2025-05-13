# Rust Controller 技術仕様書

## 1. 概要

Rust Controllerは、lambda-microservice基盤の中核コンポーネントとして、外部からのリクエストを受け取り、適切なランタイムコンテナに処理を委譲する役割を担います。高性能と低レイテンシを実現するためにRust言語で実装されます。

## 2. 責務

- リクエストヘッダ（Language-Title）の解析
- 適切なランタイムコンテナの選択
- Redisキャッシュの管理（読み取り/書き込み）
- ランタイムコンテナとの通信
- 実行結果の処理
- PostgreSQLへの処理ログ永続化
- メトリクスの収集と公開

## 3. アーキテクチャ

```
                  ┌─────────────────────────────────────────┐
                  │             Rust Controller             │
┌─────────┐       │                                         │       ┌─────────────┐
│ OpenFaaS │       │ ┌─────────┐  ┌──────────┐  ┌─────────┐ │       │  Runtime    │
│ Gateway  │──────▶│ │ Request │─▶│ Workflow │─▶│ Runtime │─│──────▶│ Containers  │
└─────────┘       │ │ Parser  │  │ Manager  │  │ Selector│ │       └─────────────┘
                  │ └─────────┘  └──────────┘  └─────────┘ │             │
                  │       │            │            ▲       │             │
                  └───────┼────────────┼────────────┼───────┘             │
                          │            │            │                     │
                          ▼            ▼            │                     │
                  ┌─────────────┐    ┌─────────┐    │                     │
                  │    Redis    │    │PostgreSQL│◀───┴─────────────────────┘
                  └─────────────┘    └─────────┘
```

## 4. コンポーネント詳細

### 4.1 Request Parser

- **機能**: HTTPリクエストを解析し、Language-Titleヘッダとペイロードを抽出
- **入力**: OpenFaaS Gatewayからのリクエスト
- **出力**: 構造化されたリクエストオブジェクト
- **エラー処理**: 不正なリクエスト形式の検出と適切なエラーレスポンス

### 4.2 Workflow Manager

- **機能**: リクエスト処理の全体フローを制御
- **責務**:
  - Redisキャッシュの確認
  - キャッシュヒット時の即時レスポンス
  - ランタイム選択の委譲
  - 実行結果の処理
  - レスポンス生成
  - メトリクス収集

### 4.3 Runtime Selector

- **機能**: Language-Titleに基づいて適切なランタイムコンテナを選択
- **選択ロジック**:
  - プレフィックスマッチング（例: "nodejs-", "python-", "rust-"）
  - 設定ファイルベースのマッピング
  - 動的ディスカバリー（Kubernetes Service Discovery利用）

### 4.4 Cache Manager

- **機能**: Redisとの通信を管理
- **操作**:
  - キャッシュ読み取り
  - キャッシュ書き込み
  - TTL（有効期限）管理
  - キャッシュ無効化

### 4.5 Database Logger

- **機能**: 実行ログをPostgreSQLに永続化
- **記録項目**:
  - リクエストID
  - タイムスタンプ
  - Language-Title
  - リクエストペイロード
  - レスポンスデータ
  - 実行時間
  - ステータスコード

### 4.6 Metrics Collector

- **機能**: Prometheusメトリクスの収集と公開
- **メトリクス**:
  - リクエスト数
  - レスポンス時間
  - エラー率
  - キャッシュヒット率
  - ランタイム別実行時間

## 5. API仕様

### 5.1 内部API

#### リクエスト受信エンドポイント

```
POST /execute
Content-Type: application/json
Language-Title: {language}-{title}

{
  "params": {...},
  "context": {...}
}
```

#### ランタイムコンテナ通信

```
POST /runtime/{language}/{title}
Content-Type: application/json

{
  "params": {...},
  "context": {...}
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
