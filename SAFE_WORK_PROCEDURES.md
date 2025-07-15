# Lambda Microservice 安全な作業手順書

## 作成日: 2025年7月15日
## 目的: プロジェクト修正時にコード資産を保護し、安全な作業を実現

---

## 概要

このドキュメントは、Lambda Microserviceプロジェクトの修正作業時に、重要なロジックやコード資産を失わないための安全な作業手順を定義します。

### 保護対象
- **ソースコード**: controller/src、runtimes/*/src
- **設定ファイル**: Cargo.toml、package.json、requirements.txt
- **データベーススキーマ**: database/migrations
- **ドキュメント**: docs/、README.md
- **インフラ設定**: kubernetes/、openfaas/、envoy/

---

## 安全な作業の3つの原則

### 1. 🛡️ 事前保護 (Pre-Protection)
- 作業前に必ずバックアップを作成
- プロジェクト構造の整合性を確認
- 危険な操作の事前検出

### 2. 🔄 段階的実行 (Staged Execution)
- 一度に大きな変更を行わない
- 各段階で安全性を検証
- 問題発生時の即座の復旧

### 3. ✅ 事後検証 (Post-Verification)
- 修正後の構造確認
- コード整合性の再チェック
- 作業ログの保存

---

## 提供されるツール

### 1. プロジェクト保護スクリプト (`scripts/project_guard.sh`)
プロジェクトの構造とコード資産を保護するためのコアスクリプト

**主な機能**:
- バックアップ作成
- プロジェクト構造検証
- コード整合性チェック
- 危険な操作の検出
- バックアップからの復旧

### 2. 安全な修正ワークフロー (`scripts/safe_modification_workflow.sh`)
段階的で安全な修正作業を支援するワークフロースクリプト

**主な機能**:
- 3段階の安全確認プロセス
- 定義済み修正パターンの実行
- 自動的な安全性検証
- 緊急復旧機能

---

## 基本的な使用方法

### Windows環境での実行準備
```powershell
# Git Bashまたは WSL を使用することを推奨
# PowerShellでは一部機能が制限されます

# Git Bashでの実行例
bash scripts/project_guard.sh help
bash scripts/safe_modification_workflow.sh help
```

### Linux/macOS環境での実行準備
```bash
# 実行権限の付与
chmod +x scripts/project_guard.sh
chmod +x scripts/safe_modification_workflow.sh

# 使用方法の確認
./scripts/project_guard.sh help
./scripts/safe_modification_workflow.sh help
```

---

## 作業パターン別手順

### パターン1: Rustバージョンのダウングレード

#### 手順
```bash
# 1. 事前チェックとバックアップ
bash scripts/project_guard.sh full-check

# 2. 安全な修正の実行
bash scripts/safe_modification_workflow.sh rust-downgrade

# 3. 結果の確認
bash scripts/project_guard.sh check
```

#### 実行される修正内容
- `controller/Dockerfile`: `rust:latest` → `rust:1.75-slim`
- `runtimes/rust/Dockerfile`: `rust:1.82-slim` → `rust:1.75-slim`

### パターン2: 依存関係の修正

#### 手順
```bash
# 1. 事前チェックとバックアップ
bash scripts/project_guard.sh full-check

# 2. 依存関係の修正実行
bash scripts/safe_modification_workflow.sh dependency-fix

# 3. 結果の確認
bash scripts/project_guard.sh check
```

#### 実行される修正内容
- `tonic = "0.9.2"` → `tonic = "0.8.3"`
- Kubernetes関連クレートの一時的な無効化
- WebAssembly関連クレートの一時的な無効化

### パターン3: 最小構成でのビルドテスト

#### 手順
```bash
# 1. 事前チェックとバックアップ
bash scripts/project_guard.sh full-check

# 2. 最小構成ビルドの実行
bash scripts/safe_modification_workflow.sh minimal-build

# 3. 結果の確認
docker-compose ps
```

#### 実行される内容
- 既存コンテナの安全な停止
- PostgreSQL、Redisの優先ビルド
- Node.js、Pythonランタイムの段階的ビルド

### パターン4: カスタム修正（対話式）

#### 手順
```bash
# 1. 事前チェックとバックアップ
bash scripts/project_guard.sh full-check

# 2. カスタム修正の実行（対話式）
bash scripts/safe_modification_workflow.sh custom

# 3. 結果の確認
bash scripts/project_guard.sh check
```

---

## 緊急時の対応

### 問題が発生した場合

#### 1. 即座の作業停止
```bash
# 現在の作業を停止
docker-compose down
```

#### 2. 最新バックアップからの復旧
```bash
# 利用可能なバックアップの確認
ls -la backups/

# 最新バックアップからの復旧
bash scripts/project_guard.sh restore backups/20250715_HHMMSS
```

#### 3. プロジェクト構造の再確認
```bash
# 復旧後の構造確認
bash scripts/project_guard.sh check
```

### 自動復旧機能
安全な修正ワークフローには自動復旧機能が組み込まれています：
- 修正後の安全性検証で問題が検出された場合
- ユーザーの確認を得て自動的にバックアップから復旧
- 作業ログに復旧の記録を保存

---

## ファイル構造とバックアップ

### バックアップの保存場所
```
backups/
├── 20250715_112300/     # タイムスタンプ付きバックアップ
│   ├── controller/
│   ├── runtimes/
│   ├── database/
│   ├── docs/
│   ├── git_status.txt   # Git状態の記録
│   ├── git_diff.txt     # 差分の記録
│   └── git_log.txt      # コミット履歴
└── 20250715_114500/
    └── ...
```

### 作業ログの保存場所
```
work_logs/
├── session_20250715_112300.log              # 進行中の作業ログ
├── completed_session_20250715_112300.log    # 完了した作業ログ
└── ...
```

---

## 作業ログの内容

各作業セッションで以下の情報が記録されます：

```
[2025-07-15 11:23:00] 作業セッション開始: rust-downgrade
[2025-07-15 11:23:05] 前提条件チェック完了
[2025-07-15 11:23:10] Phase 1完了: 作業前安全確認
[2025-07-15 11:23:15] コントローラーDockerfile修正: rust:latest -> rust:1.75-slim
[2025-07-15 11:23:20] RustランタイムDockerfile修正: rust:1.82-slim -> rust:1.75-slim
[2025-07-15 11:23:25] 修正後安全性検証成功
[2025-07-15 11:23:30] Phase 2完了: 段階的修正 (rust-downgrade)
[2025-07-15 11:23:35] Phase 3完了: 作業後検証と清理
[2025-07-15 11:23:40] 作業セッション完了
```

---

## トラブルシューティング

### よくある問題と解決策

#### 1. スクリプトの実行権限エラー
**Windows環境**:
```powershell
# Git Bashを使用
bash scripts/project_guard.sh help
```

**Linux/macOS環境**:
```bash
chmod +x scripts/*.sh
```

#### 2. バックアップディレクトリが見つからない
```bash
# バックアップディレクトリの手動作成
mkdir -p backups
```

#### 3. Docker関連のエラー
```bash
# Dockerサービスの確認
docker --version
docker-compose --version

# Docker Desktopの起動確認（Windows/macOS）
```

#### 4. Git関連の警告
```bash
# 未コミット変更の確認
git status

# 変更のコミットまたはスタッシュ
git add .
git commit -m "作業前のコミット"
# または
git stash
```

---

## ベストプラクティス

### 作業前の準備
1. **環境の確認**: Docker、Git、必要なツールが利用可能か確認
2. **変更のコミット**: 未コミットの変更を事前にコミット
3. **バックアップの確認**: 十分なディスク容量があることを確認

### 作業中の注意点
1. **段階的な実行**: 一度に複数の修正を行わない
2. **ログの確認**: 各段階でログを確認し、問題がないことを確認
3. **テストの実行**: 可能な限り各段階でテストを実行

### 作業後の確認
1. **構造の検証**: プロジェクト構造が正常であることを確認
2. **機能の確認**: 基本的な機能が動作することを確認
3. **ログの保存**: 作業ログを適切に保存

---

## 今後の拡張予定

### 計画中の機能
1. **自動テスト統合**: 修正後の自動テスト実行
2. **CI/CD統合**: 継続的インテグレーションとの連携
3. **通知機能**: 作業完了やエラーの通知
4. **レポート生成**: 作業結果の詳細レポート生成

### 改善予定
1. **Windows PowerShell対応**: ネイティブPowerShellサポート
2. **GUI版の提供**: グラフィカルインターフェースの提供
3. **設定のカスタマイズ**: 保護対象やバックアップ設定のカスタマイズ

---

## サポートとフィードバック

### 問題報告
- 作業ログファイルを添付して報告
- 実行環境の詳細を含める
- 再現手順を明確に記載

### 改善提案
- 新しい修正パターンの提案
- 安全性向上のアイデア
- 使いやすさの改善提案

---

**最終更新**: 2025年7月15日  
**バージョン**: 1.0.0  
**対応環境**: Windows (Git Bash), Linux, macOS
