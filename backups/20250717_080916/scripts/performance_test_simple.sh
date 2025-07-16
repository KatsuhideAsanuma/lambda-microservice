#!/bin/sh

# シンプルなパフォーマンステストスクリプト
# Alpine Linux環境で実行可能

set -e

# Configuration
CONTROLLER_URL="http://localhost:8080"
TEST_REQUESTS=50
CONCURRENT_REQUESTS=5

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}⚡ Lambda Microservice パフォーマンステスト${NC}"
echo "=============================================="

# Check if services are running
check_services() {
    echo -e "${YELLOW}📋 サービス状態を確認中...${NC}"
    
    if ! curl -s --connect-timeout 5 "$CONTROLLER_URL/health" > /dev/null 2>&1; then
        echo -e "${RED}❌ Controller service is not running at $CONTROLLER_URL${NC}"
        echo "Please start the services with: docker-compose up -d"
        return 1
    fi
    
    echo -e "${GREEN}✅ Controller service is running${NC}"
    return 0
}

# Health check performance test
health_check_test() {
    echo -e "${YELLOW}🏥 ヘルスチェックエンドポイントのパフォーマンステスト...${NC}"
    
    local total_time=0
    local success_count=0
    local error_count=0
    
    for i in $(seq 1 $TEST_REQUESTS); do
        local start_time=$(date +%s.%N)
        
        if curl -s --connect-timeout 5 --max-time 10 "$CONTROLLER_URL/health" > /dev/null 2>&1; then
            local end_time=$(date +%s.%N)
            local response_time=$(echo "$end_time - $start_time" | bc -l)
            total_time=$(echo "$total_time + $response_time" | bc -l)
            success_count=$((success_count + 1))
        else
            error_count=$((error_count + 1))
        fi
        
        # Progress indicator
        if [ $((i % 10)) -eq 0 ]; then
            echo -n "."
        fi
    done
    
    echo ""
    
    if [ $success_count -gt 0 ]; then
        local avg_time=$(echo "scale=3; $total_time / $success_count" | bc -l)
        local success_rate=$(echo "scale=2; $success_count * 100 / $TEST_REQUESTS" | bc -l)
        
        echo -e "${GREEN}✅ ヘルスチェック結果:${NC}"
        echo "   - 総リクエスト数: $TEST_REQUESTS"
        echo "   - 成功数: $success_count"
        echo "   - エラー数: $error_count"
        echo "   - 成功率: ${success_rate}%"
        echo "   - 平均レスポンス時間: ${avg_time}秒"
        
        # Performance evaluation
        if [ $(echo "$avg_time < 0.1" | bc -l) -eq 1 ]; then
            echo -e "${GREEN}   - 評価: 優秀 (< 0.1秒)${NC}"
        elif [ $(echo "$avg_time < 0.5" | bc -l) -eq 1 ]; then
            echo -e "${YELLOW}   - 評価: 良好 (< 0.5秒)${NC}"
        else
            echo -e "${RED}   - 評価: 改善が必要 (>= 0.5秒)${NC}"
        fi
    else
        echo -e "${RED}❌ ヘルスチェックテストが失敗しました${NC}"
        return 1
    fi
}

# Basic load test
basic_load_test() {
    echo -e "${YELLOW}📊 基本負荷テスト...${NC}"
    
    local test_payload='{
        "language": "nodejs",
        "code": "module.exports = async () => ({ result: new Date().toISOString() });"
    }'
    
    local total_time=0
    local success_count=0
    local error_count=0
    
    echo "実行中のテスト: $TEST_REQUESTS リクエスト"
    
    for i in $(seq 1 $TEST_REQUESTS); do
        local start_time=$(date +%s.%N)
        
        local response=$(curl -s --connect-timeout 10 --max-time 30 \
            -X POST "$CONTROLLER_URL/execute" \
            -H "Content-Type: application/json" \
            -H "Language-Title: nodejs-test" \
            -d "$test_payload" 2>/dev/null)
        
        if [ $? -eq 0 ] && echo "$response" | grep -q "result" 2>/dev/null; then
            local end_time=$(date +%s.%N)
            local response_time=$(echo "$end_time - $start_time" | bc -l)
            total_time=$(echo "$total_time + $response_time" | bc -l)
            success_count=$((success_count + 1))
        else
            error_count=$((error_count + 1))
        fi
        
        # Progress indicator
        if [ $((i % 5)) -eq 0 ]; then
            echo -n "."
        fi
    done
    
    echo ""
    
    if [ $success_count -gt 0 ]; then
        local avg_time=$(echo "scale=3; $total_time / $success_count" | bc -l)
        local success_rate=$(echo "scale=2; $success_count * 100 / $TEST_REQUESTS" | bc -l)
        local throughput=$(echo "scale=2; $success_count / $total_time" | bc -l)
        
        echo -e "${GREEN}✅ 負荷テスト結果:${NC}"
        echo "   - 総リクエスト数: $TEST_REQUESTS"
        echo "   - 成功数: $success_count"
        echo "   - エラー数: $error_count"
        echo "   - 成功率: ${success_rate}%"
        echo "   - 平均レスポンス時間: ${avg_time}秒"
        echo "   - スループット: ${throughput} req/sec"
        
        # Performance evaluation
        if [ $(echo "$avg_time < 2.0" | bc -l) -eq 1 ]; then
            echo -e "${GREEN}   - 評価: 優秀 (< 2.0秒)${NC}"
        elif [ $(echo "$avg_time < 5.0" | bc -l) -eq 1 ]; then
            echo -e "${YELLOW}   - 評価: 良好 (< 5.0秒)${NC}"
        else
            echo -e "${RED}   - 評価: 改善が必要 (>= 5.0秒)${NC}"
        fi
    else
        echo -e "${RED}❌ 負荷テストが失敗しました${NC}"
        return 1
    fi
}

# Memory and resource usage test
resource_usage_test() {
    echo -e "${YELLOW}💾 リソース使用量テスト...${NC}"
    
    # Get container stats if available
    if command -v docker > /dev/null 2>&1; then
        echo "Docker コンテナの統計情報:"
        docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" 2>/dev/null || echo "Docker統計情報を取得できませんでした"
    fi
    
    echo -e "${GREEN}✅ リソース使用量テスト完了${NC}"
}

# Main execution
main() {
    echo -e "${BLUE}🚀 パフォーマンステストを開始します${NC}"
    echo ""
    
    if ! check_services; then
        echo -e "${RED}❌ サービスが利用できません。テストを中止します。${NC}"
        exit 1
    fi
    
    echo ""
    health_check_test
    
    echo ""
    basic_load_test
    
    echo ""
    resource_usage_test
    
    echo ""
    echo -e "${GREEN}🎉 パフォーマンステスト完了${NC}"
}

# Run main function
main "$@"
