#!/bin/bash

# Lambda Microservice 安全な修正作業ワークフロー
# 作成日: 2025-07-15
# 目的: プロジェクト修正時の安全な作業手順を提供

set -euo pipefail

# 色付きログ出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

# プロジェクトルートディレクトリの確認
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD_SCRIPT="${PROJECT_ROOT}/scripts/project_guard.sh"

# 作業セッション情報
WORK_SESSION_ID="$(date +%Y%m%d_%H%M%S)"
WORK_LOG="${PROJECT_ROOT}/work_logs/session_${WORK_SESSION_ID}.log"

log_info "安全な修正作業ワークフローを開始します"
log_info "セッションID: ${WORK_SESSION_ID}"

# 作業ログディレクトリの作成
mkdir -p "${PROJECT_ROOT}/work_logs"

# ログ記録関数
log_to_file() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "${WORK_LOG}"
}

# 前提条件チェック
check_prerequisites() {
    log_step "前提条件をチェックしています..."
    
    # プロジェクト保護スクリプトの存在確認
    if [[ ! -f "$GUARD_SCRIPT" ]]; then
        log_error "プロジェクト保護スクリプトが見つかりません: $GUARD_SCRIPT"
        exit 1
    fi
    
    # 実行権限の確認
    if [[ ! -x "$GUARD_SCRIPT" ]]; then
        log_info "プロジェクト保護スクリプトに実行権限を付与します"
        chmod +x "$GUARD_SCRIPT"
    fi
    
    # 必要なツールの確認
    local required_tools=("git" "docker")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            log_warning "$tool が見つかりません。一部の機能が制限される可能性があります"
        fi
    done
    
    log_success "前提条件チェックが完了しました"
    log_to_file "前提条件チェック完了"
}

# Phase 1: 作業前の安全確認
pre_work_safety_check() {
    log_step "Phase 1: 作業前の安全確認を実行しています..."
    
    # プロジェクト保護スクリプトの実行
    log_info "プロジェクト構造とコード整合性をチェックしています..."
    if ! "$GUARD_SCRIPT" check; then
        log_error "プロジェクト構造に問題があります。作業を中止します"
        log_to_file "プロジェクト構造チェック失敗 - 作業中止"
        exit 1
    fi
    
    # バックアップの作成
    log_info "作業前バックアップを作成しています..."
    if ! "$GUARD_SCRIPT" backup; then
        log_error "バックアップの作成に失敗しました。作業を中止します"
        log_to_file "バックアップ作成失敗 - 作業中止"
        exit 1
    fi
    
    # Gitの状態確認
    if [[ -d "${PROJECT_ROOT}/.git" ]]; then
        cd "$PROJECT_ROOT"
        if ! git diff --quiet || ! git diff --cached --quiet; then
            log_warning "未コミットの変更があります"
            read -p "続行しますか？ (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                log_info "作業を中止しました"
                log_to_file "ユーザーによる作業中止（未コミット変更）"
                exit 0
            fi
        fi
    fi
    
    log_success "Phase 1: 作業前の安全確認が完了しました"
    log_to_file "Phase 1完了: 作業前安全確認"
}

# Phase 2: 段階的修正の実行
execute_staged_modifications() {
    log_step "Phase 2: 段階的修正を実行しています..."
    
    local modification_type="${1:-}"
    
    case "$modification_type" in
        "rust-downgrade")
            execute_rust_version_downgrade
            ;;
        "dependency-fix")
            execute_dependency_fixes
            ;;
        "minimal-build")
            execute_minimal_build_test
            ;;
        "custom")
            execute_custom_modifications
            ;;
        *)
            log_error "不明な修正タイプ: $modification_type"
            show_modification_options
            exit 1
            ;;
    esac
    
    log_success "Phase 2: 段階的修正が完了しました"
    log_to_file "Phase 2完了: 段階的修正 ($modification_type)"
}

# Rustバージョンダウングレード
execute_rust_version_downgrade() {
    log_info "Rustバージョンのダウングレードを実行しています..."
    
    # コントローラーのDockerfile修正
    local controller_dockerfile="${PROJECT_ROOT}/controller/Dockerfile"
    if [[ -f "$controller_dockerfile" ]]; then
        log_info "コントローラーのDockerfileを修正しています..."
        sed -i.bak 's/FROM rust:latest/FROM rust:1.75-slim/' "$controller_dockerfile"
        log_to_file "コントローラーDockerfile修正: rust:latest -> rust:1.75-slim"
    fi
    
    # RustランタイムのDockerfile修正
    local rust_dockerfile="${PROJECT_ROOT}/runtimes/rust/Dockerfile"
    if [[ -f "$rust_dockerfile" ]]; then
        log_info "RustランタイムのDockerfileを修正しています..."
        sed -i.bak 's/FROM rust:1.82-slim/FROM rust:1.75-slim/' "$rust_dockerfile"
        log_to_file "RustランタイムDockerfile修正: rust:1.82-slim -> rust:1.75-slim"
    fi
    
    # 修正後の検証
    verify_modifications_safety
}

# 依存関係の修正
execute_dependency_fixes() {
    log_info "依存関係の修正を実行しています..."
    
    # 問題のあるクレートの除去・代替
    local controller_cargo="${PROJECT_ROOT}/controller/Cargo.toml"
    if [[ -f "$controller_cargo" ]]; then
        log_info "コントローラーの依存関係を修正しています..."
        # バックアップ作成
        cp "$controller_cargo" "${controller_cargo}.bak"
        
        # tonicのバージョン修正
        sed -i 's/tonic = "0.9.2"/tonic = "0.8.3"/' "$controller_cargo"
        
        # 問題のあるクレートをコメントアウト
        sed -i 's/^kube = /# kube = /' "$controller_cargo"
        sed -i 's/^k8s-openapi = /# k8s-openapi = /' "$controller_cargo"
        
        log_to_file "コントローラー依存関係修正: tonic, kube関連をコメントアウト"
    fi
    
    # Rustランタイムの依存関係修正
    local rust_cargo="${PROJECT_ROOT}/runtimes/rust/Cargo.toml"
    if [[ -f "$rust_cargo" ]]; then
        log_info "Rustランタイムの依存関係を修正しています..."
        cp "$rust_cargo" "${rust_cargo}.bak"
        
        # WebAssembly関連の一時的な無効化
        sed -i 's/^wasm-pack = /# wasm-pack = /' "$rust_cargo"
        sed -i 's/^wasmtime = /# wasmtime = /' "$rust_cargo"
        
        log_to_file "Rustランタイム依存関係修正: WebAssembly関連をコメントアウト"
    fi
    
    verify_modifications_safety
}

# 最小構成ビルドテスト
execute_minimal_build_test() {
    log_info "最小構成でのビルドテストを実行しています..."
    
    # Dockerコンテナの停止
    if command -v docker >/dev/null 2>&1; then
        log_info "既存のコンテナを停止しています..."
        docker-compose down 2>/dev/null || true
    fi
    
    # 最小構成でのビルド試行
    log_info "最小構成でのビルドを開始しています..."
    if docker-compose build --no-cache postgres redis; then
        log_success "基本サービス（PostgreSQL、Redis）のビルドが成功しました"
        log_to_file "基本サービスビルド成功"
        
        # 次にNode.js、Pythonランタイムのビルド
        if docker-compose build nodejs-runtime python-runtime; then
            log_success "Node.js、Pythonランタイムのビルドが成功しました"
            log_to_file "Node.js、Pythonランタイムビルド成功"
        else
            log_warning "Node.js、Pythonランタイムのビルドに問題があります"
            log_to_file "Node.js、Pythonランタイムビルド失敗"
        fi
    else
        log_error "基本サービスのビルドに失敗しました"
        log_to_file "基本サービスビルド失敗"
        return 1
    fi
}

# カスタム修正の実行
execute_custom_modifications() {
    log_info "カスタム修正を実行します"
    log_info "この機能は対話的に実行されます"
    
    echo "実行したい修正を選択してください:"
    echo "1) ファイルの編集"
    echo "2) 設定の変更"
    echo "3) スクリプトの実行"
    echo "4) キャンセル"
    
    read -p "選択 (1-4): " -n 1 -r
    echo
    
    case $REPLY in
        1)
            read -p "編集するファイルのパスを入力してください: " file_path
            if [[ -f "${PROJECT_ROOT}/${file_path}" ]]; then
                cp "${PROJECT_ROOT}/${file_path}" "${PROJECT_ROOT}/${file_path}.bak"
                log_info "ファイルのバックアップを作成しました: ${file_path}.bak"
                log_info "ファイルを編集してください: ${PROJECT_ROOT}/${file_path}"
                log_to_file "カスタム修正: ファイル編集 - $file_path"
            else
                log_error "ファイルが見つかりません: $file_path"
            fi
            ;;
        2)
            log_info "設定変更機能は今後実装予定です"
            ;;
        3)
            log_info "スクリプト実行機能は今後実装予定です"
            ;;
        4)
            log_info "カスタム修正をキャンセルしました"
            return 0
            ;;
        *)
            log_error "無効な選択です"
            return 1
            ;;
    esac
}

# 修正後の安全性検証
verify_modifications_safety() {
    log_info "修正後の安全性を検証しています..."
    
    # プロジェクト構造の再確認
    if ! "$GUARD_SCRIPT" check; then
        log_error "修正後のプロジェクト構造に問題があります"
        log_to_file "修正後安全性検証失敗"
        
        # 自動復旧の提案
        read -p "バックアップから復旧しますか？ (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            execute_emergency_restore
        fi
        return 1
    fi
    
    log_success "修正後の安全性検証が完了しました"
    log_to_file "修正後安全性検証成功"
}

# Phase 3: 作業後の検証と清理
post_work_verification() {
    log_step "Phase 3: 作業後の検証と清理を実行しています..."
    
    # 最終的な構造確認
    log_info "最終的なプロジェクト構造を確認しています..."
    "$GUARD_SCRIPT" check
    
    # 作業ログの保存
    log_info "作業ログを保存しています..."
    cp "$WORK_LOG" "${PROJECT_ROOT}/work_logs/completed_session_${WORK_SESSION_ID}.log"
    
    # 一時ファイルの清理
    log_info "一時ファイルを清理しています..."
    find "$PROJECT_ROOT" -name "*.bak" -type f -delete 2>/dev/null || true
    
    log_success "Phase 3: 作業後の検証と清理が完了しました"
    log_to_file "Phase 3完了: 作業後検証と清理"
}

# 緊急復旧
execute_emergency_restore() {
    log_warning "緊急復旧を実行しています..."
    
    # 最新のバックアップを検索
    local latest_backup
    latest_backup=$(find "${PROJECT_ROOT}/backups" -type d -name "20*" | sort -r | head -n 1)
    
    if [[ -n "$latest_backup" ]]; then
        log_info "最新のバックアップから復旧しています: $latest_backup"
        "$GUARD_SCRIPT" restore "$latest_backup"
        log_success "緊急復旧が完了しました"
        log_to_file "緊急復旧実行: $latest_backup"
    else
        log_error "利用可能なバックアップが見つかりません"
        log_to_file "緊急復旧失敗: バックアップなし"
    fi
}

# 修正オプションの表示
show_modification_options() {
    echo ""
    echo "利用可能な修正タイプ:"
    echo "  rust-downgrade  - Rustバージョンのダウングレード"
    echo "  dependency-fix  - 依存関係の修正"
    echo "  minimal-build   - 最小構成でのビルドテスト"
    echo "  custom          - カスタム修正（対話式）"
    echo ""
    echo "使用例:"
    echo "  $0 rust-downgrade"
    echo "  $0 dependency-fix"
}

# メイン実行関数
main() {
    local modification_type="${1:-}"
    
    if [[ -z "$modification_type" ]]; then
        log_error "修正タイプを指定してください"
        show_modification_options
        exit 1
    fi
    
    # 作業ログの開始
    log_to_file "作業セッション開始: $modification_type"
    
    # 安全な作業ワークフローの実行
    check_prerequisites
    pre_work_safety_check
    execute_staged_modifications "$modification_type"
    post_work_verification
    
    log_success "安全な修正作業ワークフローが完了しました"
    log_info "作業ログ: $WORK_LOG"
    log_to_file "作業セッション完了"
}

# エラーハンドリング
trap 'log_error "スクリプトが異常終了しました"; log_to_file "スクリプト異常終了"; exit 1' ERR

# ヘルプ表示
if [[ "${1:-}" == "help" ]] || [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    echo "Lambda Microservice 安全な修正作業ワークフロー"
    echo ""
    echo "使用方法: $0 <modification_type>"
    echo ""
    show_modification_options
    exit 0
fi

# スクリプト実行
main "$@"
