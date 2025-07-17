# Lambda Microservice

高速ラムダマイクロサービス基盤 - 本番環境対応の多言語コード実行プラットフォーム

## 概要

Lambda Microserviceは、Node.js、Python、Rustの複数言語でコードを安全かつ高速に実行できるマイクロサービス基盤です。RESTful APIを通じて上位サービスから利用でき、セッション管理、エラーハンドリング、ログ記録、メトリクス収集などの本番環境に必要な機能を提供します。

## 🚀 クイックスタート

### 1. システム起動

```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
docker-compose up -d
```

### 2. 動作確認

```bash
# ヘルスチェック
curl http://localhost:8080/health

# 簡単な計算実行
curl -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: nodejs-calculator" \
  -H "Content-Type: application/json" \
  -d '{"context":{"env":"production"},"script_content":"return event.params.a + event.params.b;"}'

# レスポンスからrequest_idを取得して実行
curl -X POST http://localhost:8080/api/v1/execute/{request_id} \
  -H "Content-Type: application/json" \
  -d '{"params":{"a":5,"b":3}}'
```

## 🏗️ アーキテクチャ

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   上位サービス    │───▶│  Lambda Service  │───▶│  ランタイム群    │
│                 │    │  (Rust Controller)│    │                 │
│ - Web API       │    │                  │    │ - Node.js       │
│ - Mobile App    │    │ - セッション管理   │    │ - Python        │
│ - Batch Job     │    │ - ルーティング     │    │ - Rust          │
│ - Webhook       │    │ - ログ記録        │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │   データストア    │
                       │                  │
                       │ - PostgreSQL     │
                       │ - Redis          │
                       └──────────────────┘
```

### 主要コンポーネント

- **Rustコントローラ**: リクエストを処理し、適切なランタイムにルーティング
- **実行ランタイム**: Node.js、Python、Rustの各言語環境
- **データストア**: PostgreSQL（ログ永続化）、Redis（キャッシュ）
- **監視**: Prometheus、Grafana、Elastic Stack

## 📋 上位サービス向け統合ガイド

### 基本的な使用パターン

#### 1. 単発実行パターン
```bash
# セッション初期化 → 実行 → 結果取得
SESSION_ID=$(curl -s -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: nodejs-calculator" \
  -H "Content-Type: application/json" \
  -d '{"context":{"env":"production"},"script_content":"return event.params.a + event.params.b;"}' \
  | jq -r '.request_id')

RESULT=$(curl -s -X POST http://localhost:8080/api/v1/execute/$SESSION_ID \
  -H "Content-Type: application/json" \
  -d '{"params":{"a":10,"b":5}}' \
  | jq -r '.result')

echo "計算結果: $RESULT"
```

#### 2. 複数実行パターン
```bash
# 一度初期化して複数回実行
SESSION_ID=$(curl -s -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: python-textprocessor" \
  -H "Content-Type: application/json" \
  -d '{"context":{"env":"production"},"script_content":"result = len(params[\"text\"].split())"}' \
  | jq -r '.request_id')

# 複数のテキストを処理
for text in "Hello World" "Lambda Microservice" "Production Ready"; do
  WORD_COUNT=$(curl -s -X POST http://localhost:8080/api/v1/execute/$SESSION_ID \
    -H "Content-Type: application/json" \
    -d "{\"params\":{\"text\":\"$text\"}}" \
    | jq -r '.result')
  echo "$text: $WORD_COUNT words"
done
```

### 言語別実行例

#### Node.js実行例
```javascript
// 上位サービス（Node.js）からの呼び出し例
const axios = require('axios');

async function executeCalculation(a, b) {
  // セッション初期化
  const initResponse = await axios.post('http://localhost:8080/api/v1/initialize', {
    context: { env: 'production' },
    script_content: 'return event.params.a + event.params.b;'
  }, {
    headers: { 'Language-Title': 'nodejs-calculator' }
  });

  const sessionId = initResponse.data.request_id;

  // 実行
  const execResponse = await axios.post(`http://localhost:8080/api/v1/execute/${sessionId}`, {
    params: { a, b }
  });

  return execResponse.data.result;
}

// 使用例
executeCalculation(10, 5).then(result => console.log('結果:', result));
```

#### Python実行例
```python
# 上位サービス（Python）からの呼び出し例
import requests
import json

def execute_text_processing(text):
    # セッション初期化
    init_response = requests.post('http://localhost:8080/api/v1/initialize', 
        headers={'Language-Title': 'python-textprocessor'},
        json={
            'context': {'env': 'production'},
            'script_content': 'result = params["text"].upper()'
        }
    )
    
    session_id = init_response.json()['request_id']
    
    # 実行
    exec_response = requests.post(f'http://localhost:8080/api/v1/execute/{session_id}',
        json={'params': {'text': text}}
    )
    
    return exec_response.json()['result']

# 使用例
result = execute_text_processing("hello world")
print(f"結果: {result}")
```

### エラーハンドリング

```bash
# エラーハンドリングの例
response=$(curl -s -w "%{http_code}" -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: invalid-language" \
  -H "Content-Type: application/json" \
  -d '{"context":{},"script_content":"test"}')

http_code="${response: -3}"
body="${response%???}"

if [ "$http_code" != "200" ]; then
  echo "エラー発生: HTTP $http_code"
  echo "詳細: $body"
  exit 1
fi
```

### パフォーマンス最適化

#### セッション再利用
```bash
# セッションを再利用して高速化
SESSION_ID=$(curl -s -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: nodejs-calculator" \
  -H "Content-Type: application/json" \
  -d '{"context":{"env":"production"},"script_content":"return event.params.a * event.params.b;"}' \
  | jq -r '.request_id')

# 同じセッションで複数回実行（高速）
for i in {1..10}; do
  curl -s -X POST http://localhost:8080/api/v1/execute/$SESSION_ID \
    -H "Content-Type: application/json" \
    -d "{\"params\":{\"a\":$i,\"b\":2}}" \
    | jq -r '.result'
done
```

#### 並列実行
```bash
# 複数セッションでの並列実行
for i in {1..5}; do
  (
    SESSION_ID=$(curl -s -X POST http://localhost:8080/api/v1/initialize \
      -H "Language-Title: nodejs-calculator" \
      -H "Content-Type: application/json" \
      -d '{"context":{"env":"production"},"script_content":"return event.params.x * 2;"}' \
      | jq -r '.request_id')
    
    curl -s -X POST http://localhost:8080/api/v1/execute/$SESSION_ID \
      -H "Content-Type: application/json" \
      -d "{\"params\":{\"x\":$i}}" \
      | jq -r '.result'
  ) &
done
wait
```

## 🔧 運用・監視

### ヘルスチェック
```bash
# システム全体のヘルスチェック
curl http://localhost:8080/health

# 各ランタイムのヘルスチェック
curl http://localhost:8081/health  # Node.js
curl http://localhost:8082/health  # Python
curl http://localhost:8083/health  # Rust
```

### ログ監視
```bash
# リアルタイムログ監視
docker-compose logs -f controller

# エラーログの確認
docker exec -it lambda-microservice-postgres-1 psql -U postgres -d lambda_microservice \
  -c "SELECT * FROM public.error_logs ORDER BY created_at DESC LIMIT 10;"

# 実行ログの確認
docker exec -it lambda-microservice-postgres-1 psql -U postgres -d lambda_microservice \
  -c "SELECT request_id, language_title, status_code, duration_ms FROM public.request_logs ORDER BY created_at DESC LIMIT 10;"
```

### メトリクス
```bash
# システムメトリクス（将来実装予定）
curl http://localhost:8080/metrics
```

## 🧪 テスト

### 統合テスト実行
```bash
# 全機能テスト
bash test_api_functions.sh

# 個別テスト
bash test_simple_api.sh
bash test_direct_api.sh
```

### カスタムテスト作成
```bash
# カスタムテスト例
#!/bin/bash
echo "=== カスタム関数テスト ==="

# 独自のスクリプトでテスト
SESSION_ID=$(curl -s -X POST http://localhost:8080/api/v1/initialize \
  -H "Language-Title: nodejs-custom" \
  -H "Content-Type: application/json" \
  -d '{
    "context": {"env": "test"},
    "script_content": "return {status: \"success\", data: event.params, timestamp: new Date().toISOString()};"
  }' | jq -r '.request_id')

RESULT=$(curl -s -X POST http://localhost:8080/api/v1/execute/$SESSION_ID \
  -H "Content-Type: application/json" \
  -d '{"params": {"user_id": 123, "action": "test"}}')

echo "結果: $RESULT"
```

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

## 🚀 本番環境デプロイ

### 環境変数設定

本番環境では以下の環境変数を適切に設定してください：

```bash
# .env.production
DATABASE_URL=postgresql://user:password@prod-db:5432/lambda_microservice
REDIS_URL=redis://prod-redis:6379
RUST_LOG=info
DB_LOGGING_ENABLED=true
SESSION_EXPIRY_SECONDS=3600
RUNTIME_TIMEOUT_SECONDS=30
MAX_SCRIPT_SIZE=1048576
```

### Docker Compose本番設定

```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  controller:
    image: lambda-microservice/controller:latest
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - RUST_LOG=info
    deploy:
      replicas: 3
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M
```

### セキュリティ考慮事項

- **ネットワーク分離**: ランタイムコンテナは内部ネットワークのみアクセス
- **リソース制限**: CPU・メモリ制限を適切に設定
- **ログ監視**: 実行ログとエラーログの継続監視
- **認証**: 本番環境では適切な認証機構を実装

### スケーリング

```bash
# 水平スケーリング
docker-compose up -d --scale controller=3 --scale nodejs-runtime=2 --scale python-runtime=2

# Kubernetes環境での自動スケーリング
kubectl apply -f kubernetes/
```

## 📊 パフォーマンス指標

### ベンチマーク結果

| 言語 | 平均実行時間 | メモリ使用量 | スループット |
|------|-------------|-------------|-------------|
| Node.js | 3-33ms | 194KB-949KB | 1000 req/s |
| Python | 0-5ms | 0-1MB | 800 req/s |
| Rust | 1-50ms | 1MB | 1200 req/s |

### 最適化のヒント

1. **セッション再利用**: 同じスクリプトを複数回実行する場合
2. **並列実行**: 独立したタスクの並列処理
3. **適切なランタイム選択**: タスクに応じた言語選択
4. **リソース監視**: CPU・メモリ使用量の継続監視

## 🔍 トラブルシューティング

### よくある問題

#### 1. セッションが見つからない
```bash
# 原因: セッション有効期限切れ
# 解決: セッション有効期限を確認し、必要に応じて延長

curl http://localhost:8080/api/v1/sessions/{request_id}
```

#### 2. ランタイムエラー
```bash
# 原因: スクリプト構文エラーまたはランタイム例外
# 解決: エラーログを確認

docker-compose logs controller
docker exec -it lambda-microservice-postgres-1 psql -U postgres -d lambda_microservice \
  -c "SELECT * FROM public.error_logs WHERE request_log_id = 'your-request-id';"
```

#### 3. パフォーマンス問題
```bash
# 原因: リソース不足またはスクリプト最適化不足
# 解決: リソース使用量とスクリプト効率を確認

docker stats
curl http://localhost:8080/api/v1/sessions/{request_id}  # 実行時間確認
```

### ログレベル設定

```bash
# デバッグモード
export RUST_LOG=debug
docker-compose restart controller

# 本番モード
export RUST_LOG=info
docker-compose restart controller
```

## 🤝 コントリビューション

### 開発環境セットアップ

```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
git checkout -b feature/your-feature
```

### テスト実行

```bash
# 全テスト実行
bash test_api_functions.sh

# 単体テスト
cd controller && cargo test
```

### プルリクエスト

1. フィーチャーブランチを作成
2. 変更を実装
3. テストを追加・実行
4. ドキュメントを更新
5. プルリクエストを作成

## 📄 ライセンス

このプロジェクトはMITライセンスの下で公開されています。詳細は[LICENSE](LICENSE)ファイルを参照してください。

## 📞 サポート

- **Issues**: [GitHub Issues](https://github.com/KatsuhideAsanuma/lambda-microservice/issues)
- **Discussions**: [GitHub Discussions](https://github.com/KatsuhideAsanuma/lambda-microservice/discussions)
- **Documentation**: [docs/](./docs/)

## 🔗 関連リンク

- [API仕様書](./docs/api/api_specification.md)
- [技術仕様書](./docs/technical/rust_controller_spec.md)
- [設計ドキュメント](./docs/)
- [Docker Hub](https://hub.docker.com/r/lambda-microservice)

---

**Lambda Microservice** - 高速・安全・スケーラブルな多言語コード実行プラットフォーム
