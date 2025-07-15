# Rust バージョン最適化・機能復旧計画

## 作成日: 2025年7月15日
## 目的: 最適なRustバージョンの決定と依存関係の再構築

---

## 1. Rustバージョン分析と決定

### 現在の問題分析
- **現在のRust**: 1.51.0 (2021-03-23) - 非常に古い
- **Docker指定**: rust:1.70-slim - 中途半端なバージョン
- **要求バージョン**: 
  - tinystr v0.8.1 → Rust 1.81+
  - zerotrie v0.2.2 → Rust 1.82+
  - zerovec v0.11.2 → Rust 1.82+

### 推奨Rustバージョン: **1.75.0**

#### 選定理由
1. **安定性**: 2024年1月リリース、十分に安定
2. **互換性**: 現代的なクレートとの互換性を確保
3. **LTS的位置**: 長期サポートが期待できる
4. **依存関係**: 主要クレートが対応済み
5. **パフォーマンス**: 最新の最適化を含む

---

## 2. 依存関係再構築計画

### 2.1 Web Framework (Actix-Web)
```toml
# 現在: actix-web = "3.3" (古い)
# 新規: 
actix-web = "4.4"
actix-cors = "0.7"
actix-rt = "2.9"
```

### 2.2 データベース (PostgreSQL)
```toml
# 現在: tokio-postgres = "0.7.0" + deadpool-postgres = "0.10.0"
# 新規: SQLxに統一
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "json"] }
```

### 2.3 Redis/キャッシュ
```toml
# 現在: インメモリキャッシュで代替
# 新規: 
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.14"
```

### 2.4 WebAssembly
```toml
# 現在: 無効化
# 新規:
wasmtime = "15.0"
wasmtime-wasi = "15.0"
wasm-pack = "0.12" # build dependency
```

### 2.5 gRPC
```toml
# 現在: 無効化
# 新規:
tonic = "0.10"
prost = "0.12"
```

### 2.6 Kubernetes
```toml
# 現在: 無効化
# 新規:
kube = { version = "0.87", features = ["runtime", "derive"] }
k8s-openapi = { version = "0.20", features = ["v1_28"] }
```

### 2.7 非同期ランタイム
```toml
# 現在: tokio = "1.28.2"
# 新規:
tokio = { version = "1.35", features = ["full"] }
futures = "0.3"
```

### 2.8 ログ・トレーシング
```toml
# 現在: tracing = "0.1.37", tracing-actix-web無効化
# 新規:
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-actix-web = "0.7"
```

---

## 3. 段階的実装計画

### Phase A: 基盤更新 (1-2日)
1. **Dockerfileの更新**
   ```dockerfile
   FROM rust:1.75-slim as builder
   ```

2. **Cargo.tomlの全面書き換え**
   - 新しい依存関係バージョンに更新
   - 不要な依存関係の削除
   - フィーチャーフラグの整理

3. **ビルド確認**
   - 基本的なコンパイル確認
   - 警告・エラーの修正

### Phase B: コア機能復旧 (2-3日)
1. **データベース機能**
   - SQLxへの移行
   - 接続プール設定の調整
   - クエリの互換性確認

2. **Redis機能**
   - インメモリキャッシュからRedisへの復旧
   - 接続設定の調整
   - キャッシュロジックの確認

3. **HTTP API**
   - Actix-Web 4.xへの移行
   - ミドルウェアの更新
   - CORS設定の調整

### Phase C: 高度機能復旧 (3-4日)
1. **WebAssembly機能**
   - Wasmtime 15.xへの移行
   - WASI機能の実装
   - コンパイル・実行フローの復旧

2. **gRPC機能**
   - Tonic 0.10への移行
   - プロトコル定義の更新
   - クライアント・サーバー実装

3. **Kubernetes機能**
   - Kube 0.87への移行
   - サービス発見の実装
   - 動的ルーティングの復旧

### Phase D: テスト・検証 (1-2日)
1. **単体テスト**
   - 全モジュールのテスト実行
   - モック実装の更新

2. **統合テスト**
   - データベース統合テスト
   - Redis統合テスト
   - API統合テスト

3. **E2Eテスト**
   - 全機能の動作確認
   - パフォーマンステスト

---

## 4. 自作が必要な機能の特定

### 4.1 軽量WebAssembly実行エンジン
**理由**: Wasmtimeが重すぎる場合の代替
**実装方針**:
```rust
// 軽量WASM実行エンジン
pub struct LightWasmEngine {
    // wasmer-coreを使用した軽量実装
}

impl LightWasmEngine {
    pub async fn execute(&self, wasm_bytes: &[u8], params: Value) -> Result<Value> {
        // 基本的なWASM実行のみ実装
    }
}
```

### 4.2 簡易gRPCクライアント
**理由**: Tonicが複雑すぎる場合の代替
**実装方針**:
```rust
// HTTP/2ベースの簡易gRPCクライアント
pub struct SimpleGrpcClient {
    client: reqwest::Client,
}

impl SimpleGrpcClient {
    pub async fn call(&self, service: &str, method: &str, payload: &[u8]) -> Result<Vec<u8>> {
        // HTTP/2 + Protobufでの基本的なgRPC通信
    }
}
```

### 4.3 軽量Kubernetesクライアント
**理由**: Kubeクレートが重すぎる場合の代替
**実装方針**:
```rust
// REST APIベースの軽量K8sクライアント
pub struct LightK8sClient {
    client: reqwest::Client,
    base_url: String,
}

impl LightK8sClient {
    pub async fn list_services(&self, namespace: &str) -> Result<Vec<Service>> {
        // Kubernetes REST APIを直接呼び出し
    }
}
```

---

## 5. 実装優先度

### 最高優先度 (必須)
1. **HTTP API基盤** - Actix-Web 4.x
2. **データベース** - SQLx
3. **基本ランタイム実行** - Node.js/Python

### 高優先度 (重要)
1. **Redis機能** - キャッシュ・セッション
2. **WebAssembly** - Rust実行環境
3. **ログ・トレーシング** - 監視基盤

### 中優先度 (有用)
1. **gRPC機能** - マイクロサービス通信
2. **Kubernetes統合** - 動的サービス発見

### 低優先度 (将来)
1. **高度な監視** - Prometheus/Grafana
2. **セキュリティ強化** - 詳細な認証・認可

---

## 6. リスク評価と対策

### 高リスク
1. **WebAssembly互換性**
   - **対策**: 段階的移行、フォールバック実装
   - **自作オプション**: 軽量WASM実行エンジン

2. **gRPC複雑性**
   - **対策**: 最小限の機能実装
   - **自作オプション**: HTTP/2ベース簡易実装

### 中リスク
1. **Kubernetes API変更**
   - **対策**: バージョン固定、互換性テスト
   - **自作オプション**: REST API直接呼び出し

2. **パフォーマンス劣化**
   - **対策**: ベンチマークテスト、最適化

### 低リスク
1. **データベース移行**
   - **対策**: 段階的移行、テスト強化

2. **Redis接続**
   - **対策**: 接続プール設定調整

---

## 7. 成功基準

### 技術的成功基準
1. **ビルド成功率**: 100%
2. **テスト成功率**: 95%以上
3. **パフォーマンス**: 現在と同等以上
4. **メモリ使用量**: 現在の120%以下

### 機能的成功基準
1. **基本API**: 全エンドポイント動作
2. **多言語実行**: Node.js、Python、Rust対応
3. **セッション管理**: 作成・取得・更新・削除
4. **データ永続化**: PostgreSQL、Redis正常動作

---

## 8. 実装スケジュール

### Week 1: 基盤更新
- Day 1-2: Rust 1.75.0への更新、基本依存関係
- Day 3-4: HTTP API基盤（Actix-Web 4.x）
- Day 5: データベース（SQLx移行）

### Week 2: コア機能
- Day 1-2: Redis機能復旧
- Day 3-4: WebAssembly機能（または自作軽量版）
- Day 5: 基本テスト・検証

### Week 3: 高度機能
- Day 1-2: gRPC機能（または自作簡易版）
- Day 3-4: Kubernetes機能（または自作軽量版）
- Day 5: 統合テスト

### Week 4: 最終調整
- Day 1-2: パフォーマンス最適化
- Day 3-4: E2Eテスト、ドキュメント更新
- Day 5: 本番デプロイ準備

---

## 結論

**Rust 1.75.0**を基準とした段階的な機能復旧により、現代的で安定したLambda Microserviceシステムを構築できます。重要な機能については自作実装も準備し、確実な動作を保証します。
