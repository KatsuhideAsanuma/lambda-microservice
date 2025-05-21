#!/bin/bash
set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}ラムダマイクロサービスのローカル開発環境をセットアップしています...${NC}"

echo -e "\n${YELLOW}前提条件のチェック:${NC}"
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Dockerがインストールされていません。インストールが必要です。${NC}"
    exit 1
else
    echo -e "${GREEN}Docker OK${NC}"
fi

if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}Docker Composeがインストールされていません。インストールが必要です。${NC}"
    exit 1
else
    echo -e "${GREEN}Docker Compose OK${NC}"
fi

if ! command -v psql &> /dev/null; then
    echo -e "${YELLOW}警告: PostgreSQLクライアント(psql)がインストールされていません。インストールを推奨します。${NC}"
else
    echo -e "${GREEN}PostgreSQLクライアント OK${NC}"
fi

if [ ! -f .env ]; then
    echo -e "${YELLOW}ルートディレクトリに.envファイルが見つかりません。作成します...${NC}"
    cp controller/.env .env
    sed -i 's/localhost:5432/postgres:5432/g' .env
    sed -i 's/localhost:6379/redis:6379/g' .env
    sed -i 's/localhost:8081/nodejs-runtime:8081/g' .env
    sed -i 's/localhost:8082/python-runtime:8082/g' .env
    sed -i 's/localhost:8083/rust-runtime:8083/g' .env
    echo -e "${GREEN}.envファイルを作成しました${NC}"
else
    echo -e "${GREEN}.envファイルは既に存在します${NC}"
fi

echo -e "\n${YELLOW}データベースを起動してマイグレーションを実行します...${NC}"
docker-compose up -d postgres
echo "PostgreSQLの起動を待機しています..."
sleep 5

./scripts/migrate_database.sh
echo -e "${GREEN}データベースマイグレーションが完了しました${NC}"

echo -e "\n${YELLOW}サンプルデータを初期化します...${NC}"
./scripts/init_sample_data.sh
echo -e "${GREEN}サンプルデータの初期化が完了しました${NC}"

echo -e "\n${YELLOW}すべてのサービスを起動します...${NC}"
docker-compose up -d
echo -e "${GREEN}すべてのサービスが起動しました${NC}"

echo -e "\n${YELLOW}ランタイムのテストを実行します...${NC}"
echo "サービスの起動を待機しています..."
sleep 10
./scripts/test_runtimes.sh

echo -e "\n${GREEN}ローカル開発環境のセットアップが完了しました！${NC}"
echo -e "以下のコマンドでサービスの状態を確認できます: ${YELLOW}docker-compose ps${NC}"
echo -e "各ランタイムは以下のURLで利用可能です:"
echo -e "- Controller: ${YELLOW}http://localhost:8080${NC}"
echo -e "- Node.js Runtime: ${YELLOW}http://localhost:8081${NC}"
echo -e "- Python Runtime: ${YELLOW}http://localhost:8082${NC}"
echo -e "- Rust Runtime: ${YELLOW}http://localhost:8083${NC}"
echo -e "\nHave fun coding!"
