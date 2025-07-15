#!/bin/bash

# Lambda Microservice プロジェクト保護スクリプト
# 作成日: 2025-07-15
# 目的: 修正作業時にプロジェクト構造とコード資産を保護

set -euo pipefail

# 色付きログ出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# プロジェクトルートディレクトリの確認
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="${PROJECT_ROOT}/backups/$(date +%Y%m%d_%H%M%S)"

log_info "プロジェクト保護スクリプトを開始します"
log_info "プロジェクトルート: ${PROJECT_ROOT}"

# 重要なファイル・ディレクトリのリスト
CRITICAL_FILES=(
    "controller/src"
    "runtimes/nodejs/src"
    "runtimes/python/src"
    "runtimes/rust/src"
    "database/migrations"
    "docs"
    "README.md"
    "docker-compose.yml"
    "controller/Cargo.toml"
    "runtimes/rust/Cargo.toml"
    "runtimes/nodejs/package.json"
    "runtimes/python/requirements.txt"
)

# 保護対象の設定ファイル
CONFIG_FILES=(
    "kubernetes"
    "openfaas"
    "envoy"
    "scripts"
    "test"
)

# バックアップ作成関数
create_backup() {
    log_info "バックアップを作成しています..."
    mkdir -p "${BACKUP_DIR}"
    
    # 重要なファイルのバックアップ
    for file in "${CRITICAL_FILES[@]}"; do
        if [[ -e "${PROJECT_ROOT}/${file}" ]]; then
            log_info "バックアップ中: ${file}"
            cp -r "${PROJECT_ROOT}/${file}" "${BACKUP_DIR}/"
        else
            log_warning "ファイルが見つかりません: ${file}"
        fi
    done
    
    # 設定ファイルのバックアップ
    for config in "${CONFIG_FILES[@]}"; do
        if [[ -e "${PROJECT_ROOT}/${config}" ]]; then
            log_info "設定バックアップ中: ${config}"
            cp -r "${PROJECT_ROOT}/${config}" "${BACKUP_DIR}/"
        fi
    done
    
    # Gitの状態保存
    if [[ -d "${PROJECT_ROOT}/.git" ]]; then
        log_info "Git状態を保存しています..."
        cd "${PROJECT_ROOT}"
        git status > "${BACKUP_DIR}/git_status.txt" 2>&1 || true
        git diff > "${BACKUP_DIR}/git_diff.txt" 2>&1 || true
        git log --oneline -10 > "${BACKUP_DIR}/git_log.txt" 2>&1 || true
    fi
    
    log_success "バックアップが完了しました: ${BACKUP_DIR}"
}

# プロジェクト構造の検証
verify_project_structure() {
    log_info "プロジェクト構造を検証しています..."
    
    local errors=0
    
    # 重要なディレクトリの存在確認
    local required_dirs=(
        "controller"
        "runtimes"
        "database"
        "docs"
    )
    
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "${PROJECT_ROOT}/${dir}" ]]; then
            log_error "必須ディレクトリが見つかりません: ${dir}"
            ((errors++))
        fi
    done
    
    # 重要なファイルの存在確認
    local required_files=(
        "README.md"
        "docker-compose.yml"
        "INVESTIGATION_PLAN.md"
    )
    
    for file in "${required_files[@]}"; do
        if [[ ! -f "${PROJECT_ROOT}/${file}" ]]; then
            log_error "必須ファイルが見つかりません: ${file}"
            ((errors++))
        fi
    done
    
    # Rustプロジェクトの構造確認
    if [[ ! -f "${PROJECT_ROOT}/controller/Cargo.toml" ]]; then
        log_error "コントローラーのCargo.tomlが見つかりません"
        ((errors++))
    fi
    
    if [[ ! -d "${PROJECT_ROOT}/controller/src" ]]; then
        log_error "コントローラーのsrcディレクトリが見つかりません"
        ((errors++))
    fi
    
    if [[ $errors -gt 0 ]]; then
        log_error "プロジェクト構造に ${errors} 個の問題が見つかりました"
        return 1
    fi
    
    log_success "プロジェクト構造の検証が完了しました"
    return 0
}

# コード資産の整合性チェック
verify_code_integrity() {
    log_info "コード資産の整合性をチェックしています..."
    
    local warnings=0
    
    # Rustコードの基本的な構文チェック
    if command -v cargo >/dev/null 2>&1; then
        cd "${PROJECT_ROOT}/controller"
        if ! cargo check --quiet 2>/dev/null; then
            log_warning "コントローラーのRustコードに構文エラーがあります"
            ((warnings++))
        fi
        
        if [[ -d "${PROJECT_ROOT}/runtimes/rust" ]]; then
            cd "${PROJECT_ROOT}/runtimes/rust"
            if ! cargo check --quiet 2>/dev/null; then
                log_warning "Rustランタイムのコードに構文エラーがあります"
                ((warnings++))
            fi
        fi
    else
        log_warning "Cargoが見つかりません。Rustコードのチェックをスキップします"
    fi
    
    # Node.jsプロジェクトのチェック
    if [[ -f "${PROJECT_ROOT}/runtimes/nodejs/package.json" ]]; then
        if command -v npm >/dev/null 2>&1; then
            cd "${PROJECT_ROOT}/runtimes/nodejs"
            if ! npm ls >/dev/null 2>&1; then
                log_warning "Node.jsの依存関係に問題があります"
                ((warnings++))
            fi
        fi
    fi
    
    # Pythonプロジェクトのチェック
    if [[ -f "${PROJECT_ROOT}/runtimes/python/requirements.txt" ]]; then
        if command -v python3 >/dev/null 2>&1; then
            cd "${PROJECT_ROOT}/runtimes/python"
            if ! python3 -m py_compile src/*.py 2>/dev/null; then
                log_warning "Pythonコードに構文エラーがあります"
                ((warnings++))
            fi
        fi
    fi
    
    if [[ $warnings -gt 0 ]]; then
        log_warning "コード整合性チェックで ${warnings} 個の警告が見つかりました"
    else
        log_success "コード資産の整合性チェックが完了しました"
    fi
    
    return 0
}

# 危険な操作の検出
check_dangerous_operations() {
    log_info "危険な操作をチェックしています..."
    
    # 実行中のDockerコンテナの確認
    if command -v docker >/dev/null 2>&1; then
        local running_containers
        running_containers=$(docker ps --filter "name=lambda-microservice" --format "table {{.Names}}" | tail -n +2)
        
        if [[ -n "$running_containers" ]]; then
            log_warning "以下のコンテナが実行中です:"
            echo "$running_containers"
            log_warning "作業前にコンテナを停止することを推奨します"
        fi
    fi
    
    # 未コミットの変更の確認
    if [[ -d "${PROJECT_ROOT}/.git" ]]; then
        cd "${PROJECT_ROOT}"
        if ! git diff --quiet; then
            log_warning "未コミットの変更があります"
            log_warning "作業前にコミットまたはスタッシュすることを推奨します"
        fi
        
        if ! git diff --cached --quiet; then
            log_warning "ステージングされた変更があります"
        fi
    fi
}

# 復旧関数
restore_from_backup() {
    local backup_path="$1"
    
    if [[ ! -d "$backup_path" ]]; then
        log_error "バックアップディレクトリが見つかりません: $backup_path"
        return 1
    fi
    
    log_info "バックアップから復旧しています: $backup_path"
    
    # 重要なファイルの復旧
    for file in "${CRITICAL_FILES[@]}"; do
        if [[ -e "${backup_path}/${file}" ]]; then
            log_info "復旧中: ${file}"
            rm -rf "${PROJECT_ROOT}/${file}"
            cp -r "${backup_path}/${file}" "${PROJECT_ROOT}/${file}"
        fi
    done
    
    log_success "復旧が完了しました"
}

# メイン実行関数
main() {
    local command="${1:-check}"
    
    case "$command" in
        "backup")
            create_backup
            ;;
        "check")
            verify_project_structure
            verify_code_integrity
            check_dangerous_operations
            ;;
        "restore")
            if [[ -z "${2:-}" ]]; then
                log_error "復旧するバックアップパスを指定してください"
                log_info "使用方法: $0 restore <backup_path>"
                exit 1
            fi
            restore_from_backup "$2"
            ;;
        "full-check")
            create_backup
            verify_project_structure
            verify_code_integrity
            check_dangerous_operations
            ;;
        "help")
            echo "Lambda Microservice プロジェクト保護スクリプト"
            echo ""
            echo "使用方法: $0 <command>"
            echo ""
            echo "コマンド:"
            echo "  backup      - プロジェクトのバックアップを作成"
            echo "  check       - プロジェクト構造とコード整合性をチェック"
            echo "  restore     - バックアップから復旧"
            echo "  full-check  - バックアップ作成 + 完全チェック"
            echo "  help        - このヘルプを表示"
            ;;
        *)
            log_error "不明なコマンド: $command"
            log_info "使用方法: $0 help"
            exit 1
            ;;
    esac
}

# スクリプト実行
main "$@"
