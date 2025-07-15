# Lambda Microservice プロジェクト概要

## プロジェクト基本情報

### プロジェクト名
Lambda Microservice - 高速ラムダマイクロサービス基盤

### プロジェクト概要
複数のプログラミング言語（Node.js、Python、Rust）でコードを実行できるマイクロサービス基盤。WebAssembly、gRPC、Kubernetesサポートを含む包括的なランタイム環境を提供。

### 技術スタック
- **言語**: Rust (コントローラー)、Node.js、Python、WebAssembly
- **フレームワーク**: Actix-web、OpenFaaS
- **データベース**: PostgreSQL、Redis
- **インフラ**: Docker、Kubernetes、Envoy
- **監視**: Prometheus、Grafana、Elastic Stack

### アーキテクチャ
```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Envoy     │───▶│ Rust         │───▶│ Runtime         │
│ (Gateway)   │    │ Controller   │    │ Containers      │
└─────────────┘    └──────────────┘    │ - Node.js       │
                           │            │ - Python        │
                           │            │ - Rust/WASM     │
                           ▼            └─────────────────┘
                   ┌──────────────┐
                   │ PostgreSQL   │
                   │ Redis        │
                   └──────────────┘
```

---

## 現在の課題

### 🔴 緊急課題（ブロッカー）

#### 1. Rustビルド失敗問題
**問題**: 全Rustコンテナ（controller、rust-runtime）のビルドが失敗
**原因**: 
- `base64ct-1.8.0`クレートが`edition2024`機能を要求
- Cargo 1.82.0では`edition2024`がサポートされていない
- 間接依存関係により問題のクレートが強制的に引き込まれる

**影響**: 
- サービス全体が起動不可
- 開発・テスト・本番環境すべてに影響
- CI/CDパイプライン完全停止

#### 2. 依存関係の複雑性問題
**問題**: 複雑な依存関係チェーンによる制御困難
**詳細**:
- gRPC関連: `tonic = "0.9.2"`が利用不可（0.8.x系のみ利用可能）
- WebAssembly関連: `wasmtime`、`wasm-pack`の互換性問題
- Kubernetes関連: `kube`、`k8s-openapi`のバージョン競合

### 🟡 中優先度課題

#### 3. 設定管理の問題（解決済み）
- ~~Secretsファイルの欠如~~ ✅ 解決
- ~~Docker Compose設定の警告~~ ✅ 解決

---

## 解決済み項目

### ✅ 設定・環境の修正
1. **Secretsファイルの作成**:
   - `secrets/db_url.txt`: PostgreSQL接続文字列
   - `secrets/redis_url.txt`: Redis接続文字列
   - `secrets/redis_cache_url.txt`: Redisキャッシュ接続文字列

2. **Docker Compose設定の改善**:
   - 廃止予定の`version: "3"`属性を削除
   - 警告メッセージの解消

---

## 推奨解決策

### 即座に実施すべき対策
1. **Rustバージョンの大幅ダウングレード**
   ```dockerfile
   FROM rust:1.75-slim as builder  # edition2024問題回避
   ```

2. **問題クレートの除去・代替**
   ```toml
   # tonic = "0.8.3"  # 利用可能バージョンに変更
   # base64ct = "1.6.0"  # 安定版に固定
   ```

3. **段階的ビルド戦略**
   - 最小構成でのビルド成功確認
   - 機能の段階的追加（WebAssembly、gRPC、Kubernetes）

### 中期的対策
- 依存関係管理の抜本的見直し
- ビルド環境の標準化
- CI/CDパイプラインの改善

### 長期的対策
- アーキテクチャの見直し
- マイクロサービス分割による依存関係分離
- 外部サービスの活用検討

---

## プロジェクト構造

### 主要ディレクトリ
```
lambda-microservice/
├── controller/          # Rustコントローラー
│   ├── src/
│   ├── Cargo.toml
│   └── Dockerfile
├── runtimes/           # 各言語ランタイム
│   ├── nodejs/
│   ├── python/
│   └── rust/
├── database/           # データベーススキーマ
│   └── migrations/
├── kubernetes/         # K8s設定
├── openfaas/          # OpenFaaS設定
├── envoy/             # API Gateway設定
└── secrets/           # 設定ファイル（新規作成）
```

### 重要ファイル
- `docker-compose.yml`: サービス定義
- `INVESTIGATION_PLAN.md`: 調査計画書
- `INVESTIGATION_RESULTS_PHASE1_FINAL.md`: 調査結果
- `README.md`: プロジェクト説明書

---

## 開発環境セットアップ

### 前提条件
- Docker & Docker Compose
- PostgreSQL クライアント
- Rust 1.75+ (推奨)

### クイックスタート（現在は失敗）
```bash
git clone https://github.com/KatsuhideAsanuma/lambda-microservice.git
cd lambda-microservice
docker-compose up -d  # 現在はビルドエラーで失敗
```

### 期待される動作（修正後）
- Controller: http://localhost:8080
- Node.js Runtime: http://localhost:8081
- Python Runtime: http://localhost:8082
- Rust Runtime: http://localhost:8083

---

## コーディングガイド

### 安全な開発作業の原則

#### 1. 🛡️ 事前保護 (Pre-Protection)
すべての修正作業前に必ずプロジェクト保護を実行：
```bash
# 作業前の必須チェック
bash scripts/project_guard.sh full-check
```

#### 2. 🔄 段階的実行 (Staged Execution)
大きな変更を一度に行わず、段階的に実行：
```bash
# 段階的修正の例
bash scripts/safe_modification_workflow.sh rust-downgrade
bash scripts/safe_modification_workflow.sh dependency-fix
bash scripts/safe_modification_workflow.sh minimal-build
```

#### 3. ✅ 事後検証 (Post-Verification)
各修正後に必ず構造とコードの整合性を確認：
```bash
# 修正後の必須チェック
bash scripts/project_guard.sh check
```

### ガードスクリプトを利用した作業手順

#### 新機能開発時の手順
```bash
# 1. 作業前の安全確認
bash scripts/project_guard.sh full-check

# 2. 新機能の実装
# - コードの編集
# - テストの追加

# 3. 段階的テスト
bash scripts/safe_modification_workflow.sh minimal-build

# 4. 最終確認
bash scripts/project_guard.sh check
```

#### バグ修正時の手順
```bash
# 1. 現在の状態をバックアップ
bash scripts/project_guard.sh backup

# 2. 問題の特定と修正
bash scripts/safe_modification_workflow.sh custom

# 3. 修正後の検証
bash scripts/project_guard.sh check

# 4. 問題があれば即座に復旧
# bash scripts/project_guard.sh restore backups/最新バックアップ
```

#### 依存関係更新時の手順
```bash
# 1. 事前保護
bash scripts/project_guard.sh full-check

# 2. 依存関係の段階的更新
bash scripts/safe_modification_workflow.sh dependency-fix

# 3. ビルドテスト
bash scripts/safe_modification_workflow.sh minimal-build

# 4. 結果確認
docker-compose ps
bash scripts/project_guard.sh check
```

### コーディング規約

#### Rust コード
```rust
// ✅ 推奨: エラーハンドリングの明示
fn safe_operation() -> Result<String, Box<dyn std::error::Error>> {
    let result = risky_operation()?;
    Ok(result)
}

// ❌ 非推奨: unwrap()の多用
fn unsafe_operation() -> String {
    risky_operation().unwrap() // パニックの原因
}
```

#### 設定ファイル管理
```toml
# Cargo.toml - バージョン固定の推奨
[dependencies]
actix-web = "=4.3.1"  # 安定版に固定
base64ct = "=1.6.0"   # 互換性問題回避
```

#### Docker設定
```dockerfile
# 安定版Rustの使用
FROM rust:1.75-slim as builder  # edition2024問題回避

# マルチステージビルドの活用
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/app /app/
```

### 緊急時対応手順

#### ビルドエラー発生時
```bash
# 1. 即座に作業停止
docker-compose down

# 2. 最新バックアップから復旧
bash scripts/project_guard.sh restore backups/$(ls -t backups/ | head -n1)

# 3. 構造確認
bash scripts/project_guard.sh check

# 4. 問題の再調査
bash scripts/project_guard.sh full-check
```

#### コード損失の疑いがある場合
```bash
# 1. 現在の状態を一時保存
cp -r . ../emergency_backup_$(date +%Y%m%d_%H%M%S)

# 2. 利用可能なバックアップを確認
ls -la backups/

# 3. 最適なバックアップから復旧
bash scripts/project_guard.sh restore backups/選択したバックアップ

# 4. Git履歴との比較
git status
git diff
```

### 作業ログの活用

#### ログの確認方法
```bash
# 最新の作業ログを確認
tail -f work_logs/session_$(date +%Y%m%d)*.log

# 完了した作業の履歴
ls -la work_logs/completed_session_*.log
```

#### ログから問題を特定
```bash
# エラーが発生した作業セッションを検索
grep -l "エラー\|失敗\|ERROR" work_logs/*.log

# 特定の修正タイプの履歴を確認
grep "rust-downgrade\|dependency-fix" work_logs/*.log
```

### 開発環境の保守

#### 定期的なメンテナンス
```bash
# 週次: プロジェクト構造の健全性チェック
bash scripts/project_guard.sh check

# 月次: 古いバックアップの清理
find backups/ -type d -mtime +30 -exec rm -rf {} \;

# 月次: 作業ログのアーカイブ
tar -czf work_logs_archive_$(date +%Y%m).tar.gz work_logs/
```

#### 依存関係の監視
```bash
# Rustクレートの脆弱性チェック
cargo audit

# 依存関係の更新確認
cargo outdated

# Node.js依存関係のチェック（該当する場合）
cd runtimes/nodejs && npm audit
```

### チーム開発での注意点

#### 作業前の同期
```bash
# 1. 最新コードの取得
git pull origin main

# 2. プロジェクト構造の確認
bash scripts/project_guard.sh check

# 3. 他の開発者の作業ログ確認
ls -la work_logs/completed_session_$(date +%Y%m%d)*.log
```

#### 作業完了時の共有
```bash
# 1. 変更のコミット
git add .
git commit -m "feat: 機能追加 - ガードスクリプト使用"

# 2. 作業ログの保存
cp work_logs/completed_session_*.log shared_logs/

# 3. 最終確認
bash scripts/project_guard.sh check
```

---

## 連絡先・リソース

### リポジトリ
- **GitHub**: https://github.com/KatsuhideAsanuma/lambda-microservice.git
- **ブランチ**: main（現在の作業ブランチ）

### ドキュメント
- API仕様: `docs/api/api_specification.md`
- 技術仕様: `docs/technical/rust_controller_spec.md`
- データベース設計: `docs/database/database_schema.md`

### 作業ログ
- 詳細な作業履歴: `CLINE_LOG_20250715.md`
- 調査結果: `INVESTIGATION_RESULTS_PHASE1_FINAL.md`

---

**最終更新**: 2025年7月15日  
**ステータス**: 🔴 ビルド問題により開発停止中  
**次のアクション**: Phase 1.5 - 緊急ビルド修正の実施
