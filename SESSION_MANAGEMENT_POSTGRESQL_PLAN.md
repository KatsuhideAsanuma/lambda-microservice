# セッション管理 PostgreSQL単独構成 - 完全解決計画

**作成日**: 2025年7月17日  
**対象問題**: セッション管理の実装問題（「Session not found or expired」エラー）  
**解決方針**: PostgreSQL単独構成への移行による全面的な解決  

---

## 🎯 問題の現状分析

### 現在の問題
- **セッション初期化**: 成功（PostgreSQLに保存確認済み）
- **セッション検索**: 失敗（「Session not found or expired」エラー）
- **関数実行**: 0%成功率
- **システム不安定**: セッション初期化リクエストのハング

### 根本原因
1. **Redis無効化**: 「TEMPORARILY DISABLED」状態
2. **InMemoryCache不備**: 有効期限チェックが未実装
3. **二重データソース**: Redis/PostgreSQL間の不整合
4. **複雑な依存関係**: 2つのデータストアの管理コスト

---

## 💡 解決策: PostgreSQL単独構成

### 設計方針
- **Single Source of Truth**: PostgreSQLのみを使用
- **シンプルな実装**: 複雑な缶詰を排除
- **高い信頼性**: 既存の安定したPostgreSQLインフラを活用
- **運用コスト削減**: Redis依存性の完全除去

### アーキテクチャ変更
```
【現在】
SessionManager → Redis(InMemoryCache) → PostgreSQL
                    ↓ 問題箇所
               有効期限チェック不備

【変更後】
SessionManager → PostgreSQL (単独)
                    ↓ 
               完全なセッション管理
```

---

## 🔧 実装計画

### Phase 1: Redis依存性の除去（1-2時間）
1. **SessionManagerの修正**
   - RedisPoolTraitの依存関係を削除
   - PostgreSQL単独でのセッション管理実装

2. **main.rsの修正**
   - Redis初期化処理の削除
   - シンプルな依存性注入

3. **Cargo.tomlの修正**
   - redis関連依存関係の削除
   - deadpool-redis依存関係の削除

### Phase 2: PostgreSQL最適化（1時間）
1. **セッションクエリの最適化**
   - インデックスの活用
   - 期限切れセッションの効率的な削除

2. **接続プール設定の最適化**
   - 適切な接続数設定
   - タイムアウト設定の調整

### Phase 3: テストと検証（1時間）
1. **単体テスト**
   - SessionManagerのテスト実行
   - 全機能テストの実施

2. **統合テスト**
   - API統合テストの実行
   - E2Eテストの実行

---

## 📋 実装手順

### ステップ1: SessionManagerの修正
**対象ファイル**: `controller/src/session.rs`

```rust
// 修正前: Redis + PostgreSQL
pub struct SessionManager<D: DbPoolTrait, R: RedisPoolTrait> {
    db_pool: D,
    redis_pool: R,  // ← 削除
    session_expiry_seconds: u64,
}

// 修正後: PostgreSQL単独
pub struct SessionManager<D: DbPoolTrait> {
    db_pool: D,
    session_expiry_seconds: u64,
}
```

### ステップ2: 依存性注入の簡素化
**対象ファイル**: `controller/src/main.rs`

```rust
// 修正前: Redis + PostgreSQL
let redis_pool = create_redis_pool().await?;
let session_manager = SessionManager::new(db_pool.clone(), redis_pool, 3600);

// 修正後: PostgreSQL単独
let session_manager = SessionManager::new(db_pool.clone(), 3600);
```

### ステップ3: 依存関係の削除
**対象ファイル**: `controller/Cargo.toml`

```toml
# 削除対象
# redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
# deadpool-redis = "0.14"
```

### ステップ4: キャッシュ層の削除
**対象ファイル**: `controller/src/cache.rs`

```rust
// InMemoryCache実装を削除
// RedisPoolTrait実装を削除
```

---

## 🚀 期待される効果

### 即座の改善
- **セッション検索成功率**: 85.7% → **100%**
- **関数実行成功率**: 0% → **95%以上**
- **セッション初期化ハング**: 完全解決
- **システム安定性**: 大幅向上

### 長期的なメリット
- **運用コスト削減**: Redis管理不要
- **依存関係の簡素化**: トラブルシューティングが容易
- **パフォーマンス向上**: 単一データソースによる最適化
- **スケーラビリティ**: PostgreSQL固有の最適化活用

---

## 📊 成功基準

### 機能テスト
- ✅ **セッション作成**: 100%成功
- ✅ **セッション検索**: 100%成功
- ✅ **セッション更新**: 100%成功
- ✅ **セッション削除**: 100%成功

### 統合テスト
- ✅ **API統合テスト**: 100%成功
- ✅ **E2Eテスト**: 100%成功
- ✅ **負荷テスト**: 安定動作
- ✅ **パフォーマンステスト**: < 100ms

### 品質指標
- **コンパイルエラー**: 0個
- **警告**: 最小限
- **テストカバレッジ**: 95%以上
- **メモリ使用量**: 改善

---

## ⚠️ リスクと対策

### リスク1: PostgreSQL負荷増加
**対策**: 
- 適切なインデックス設定
- 接続プールの最適化
- 期限切れセッションの定期削除

### リスク2: 一時的な機能停止
**対策**:
- 段階的な移行
- 十分なテスト
- ロールバック計画

### リスク3: パフォーマンス劣化
**対策**:
- ベンチマークテスト
- PostgreSQLクエリの最適化
- 必要に応じた代替実装

---

## 🎯 実行スケジュール

### **即座開始**: Phase 1 - Redis依存性除去（1-2時間）
1. **08:10-08:30**: SessionManagerの修正
2. **08:30-08:45**: main.rsの修正
3. **08:45-09:00**: Cargo.tomlの修正
4. **09:00-09:15**: 初期ビルドテスト

### **続行**: Phase 2 - PostgreSQL最適化（1時間）
1. **09:15-09:45**: セッションクエリ最適化
2. **09:45-10:15**: 接続プール設定調整

### **完了**: Phase 3 - テストと検証（1時間）
1. **10:15-10:45**: 単体テスト実行
2. **10:45-11:15**: 統合テスト実行

### **総所要時間**: 約3時間で完全解決

---

## 📚 参考実装

### セッション管理の最適化例
```rust
impl<D: DbPoolTrait + Send + Sync> SessionManagerTrait for SessionManager<D> {
    async fn get_session(&self, request_id: &str) -> Result<Option<Session>> {
        // PostgreSQL単独でのセッション取得
        let query = r#"
            SELECT * FROM meta.sessions 
            WHERE request_id = $1 AND expires_at > NOW()
        "#;
        
        let row_opt = self.db_pool.query_opt(query, &[&request_id]).await?;
        // 実装詳細...
    }
}
```

### 期限切れセッションの自動削除
```rust
async fn cleanup_expired_sessions(&self) -> Result<u64> {
    let query = r#"
        DELETE FROM meta.sessions 
        WHERE expires_at < NOW()
    "#;
    
    self.db_pool.execute(query, &[]).await
}
```

---

## 🔄 作業状況更新（2025年7月17日 08:35）

### ✅ Phase 1完了報告
**実行期間**: 2025年7月17日 08:07-08:21 JST  
**完了内容**: Redis依存性の完全除去

#### 実装完了項目
1. **SessionManagerの修正**: `SessionManager<D, R>` → `SessionManager<D>`
2. **main.rsの修正**: Redis初期化処理削除
3. **Cargo.tomlの修正**: Redis依存関係削除
4. **error.rsの修正**: Redis関連エラーハンドリング削除
5. **lib_main.rsの修正**: Redis関連パラメータ削除

#### 成果
- **コンパイル**: 完全成功（エラー0個）
- **警告**: 26個（未使用変数のみ、機能に影響なし）
- **基本動作**: 正常（health check通過）

### ⚠️ 発見した新たな問題
**問題発生期間**: 2025年7月17日 08:21-08:35 JST

#### 発見した問題
1. **PostgreSQLクエリのハング**
   ```bash
   # ハングするクエリ
   SELECT NOW() as current_time, request_id, expires_at, (expires_at > NOW()) as is_valid 
   FROM meta.sessions WHERE request_id = 'xxx';
   ```

2. **セッション初期化のハング**
   ```bash
   # ハングするリクエスト
   curl -X POST -H "Content-Type: application/json" -H "Language-Title: nodejs-calculator" 
   -d '{"context": {"env": "test"}, "script_content": "..."}' 
   http://localhost:8080/api/v1/initialize
   ```

3. **セッション取得エラー継続**
   - PostgreSQLにセッションは正常保存
   - セッション実行時に「Session not found or expired」エラー継続

#### 根本原因分析
**推定原因**:
- **接続プール設定問題**: PostgreSQLプールのタイムアウト設定
- **クエリ性能問題**: `expires_at > NOW()`クエリの性能問題
- **時刻同期問題**: コンテナ間の時刻同期不整合
- **インデックス不足**: セッション検索の最適化不足

### 📋 更新された作業計画

#### Phase 2: PostgreSQL接続プール最適化（緊急）
**所要時間**: 1-2時間  
**優先度**: 最高

1. **接続プール設定の確認**
   - `controller/src/database.rs`の設定確認
   - タイムアウト値の調整
   - 接続数の最適化

2. **クエリの最適化**
   ```sql
   -- 問題のあるクエリ
   SELECT * FROM meta.sessions WHERE request_id = $1 AND expires_at > NOW()
   
   -- 最適化案1: インデックス活用
   SELECT * FROM meta.sessions WHERE request_id = $1 AND status = 'active'
   
   -- 最適化案2: 時刻比較を避ける
   SELECT * FROM meta.sessions WHERE request_id = $1 AND status = 'active' LIMIT 1
   ```

3. **インデックス追加**
   ```sql
   CREATE INDEX IF NOT EXISTS idx_sessions_request_id ON meta.sessions(request_id);
   CREATE INDEX IF NOT EXISTS idx_sessions_status ON meta.sessions(status);
   CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON meta.sessions(expires_at);
   ```

#### Phase 3: デバッグ強化と代替実装（中優先度）
**所要時間**: 1-2時間

1. **ログ強化**
   - セッション取得時の詳細ログ
   - PostgreSQLクエリ実行時間計測
   - エラー発生時の詳細情報

2. **代替実装の検討**
   - 時刻比較を避けたセッション管理
   - シンプルなセッション有効性チェック
   - 段階的な機能復旧

### 🎯 継続作業の成功基準

#### 緊急解決目標
- **PostgreSQLクエリハング**: 0件
- **セッション初期化ハング**: 0件
- **セッション取得成功率**: 100%
- **関数実行成功率**: 95%以上

#### 性能目標
- **セッション取得時間**: < 100ms
- **PostgreSQLクエリ時間**: < 50ms
- **API応答時間**: < 200ms

### 🔄 環境移行情報

#### 現在のシステム状態
- **Docker環境**: 正常動作中
- **PostgreSQL**: 正常動作、データ保存済み
- **基本API**: 正常動作
- **コンパイル**: 成功（警告のみ）

#### 継続作業コマンド
```bash
# 基本状態確認
docker-compose ps
curl -s http://localhost:8080/health

# セッション管理テスト
bash test_api_functions.sh

# PostgreSQL接続確認
docker exec -it lambda-microservice-postgres-1 psql -U postgres -d lambda_microservice -c "SELECT 1;"

# コンパイル確認
cd controller && cargo check
```

#### 最優先作業
1. **`controller/src/database.rs`の接続プール設定確認**
2. **PostgreSQLクエリの最適化**
3. **インデックス追加**
4. **セッション取得ロジックの見直し**

---

**計画承認者**: CLINE AI Assistant  
**実行責任者**: CLINE AI Assistant  
**Phase 1完了**: 2025年7月17日 08:21 JST ✅  
**環境移行**: 2025年7月17日 08:35 JST  
**継続作業**: PostgreSQLクエリハング問題の解決が最優先  
**最終更新**: 2025年7月17日 08:35 JST
