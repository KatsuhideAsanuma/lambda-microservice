# 運用マニュアル

## 1. 概要

本マニュアルは、lambda-microservice基盤の運用管理に関する手順と指針を提供します。システム監視、障害対応、バックアップ・リストア、スケーリング、セキュリティ管理、およびメンテナンス作業について詳細に説明します。

## 2. システム構成概要

lambda-microservice基盤は以下のコンポーネントで構成されています：

- **Envoy Proxy**: 外部からのリクエストを受け付けるAPIゲートウェイ
- **OpenFaaS Gateway**: FaaS（Function as a Service）プラットフォーム
- **Rustコントローラ**: リクエスト処理とランタイム選択を担当
- **ランタイムコンテナ**: Node.js、Python、Rustの実行環境
- **Redis**: キャッシュストレージ
- **PostgreSQL**: ログと永続データの保存
- **Prometheus/Grafana**: メトリクス収集と可視化
- **Elastic Stack**: ログ収集と分析

## 3. 監視体制

### 3.1 監視対象とメトリクス

| コンポーネント | 監視項目 | 閾値 | アラート優先度 |
|--------------|---------|------|--------------|
| Envoy Proxy | CPU使用率 | > 80% | 中 |
| Envoy Proxy | メモリ使用率 | > 80% | 中 |
| Envoy Proxy | エラーレート | > 1% | 高 |
| Rustコントローラ | CPU使用率 | > 70% | 中 |
| Rustコントローラ | メモリ使用率 | > 80% | 中 |
| Rustコントローラ | レスポンス時間 | > 50ms | 中 |
| Rustコントローラ | エラーレート | > 0.5% | 高 |
| ランタイムコンテナ | CPU使用率 | > 80% | 中 |
| ランタイムコンテナ | メモリ使用率 | > 80% | 中 |
| ランタイムコンテナ | 実行時間 | > 1000ms | 低 |
| Redis | メモリ使用率 | > 80% | 高 |
| Redis | 接続数 | > 5000 | 中 |
| Redis | レイテンシ | > 10ms | 中 |
| PostgreSQL | CPU使用率 | > 70% | 中 |
| PostgreSQL | ディスク使用率 | > 80% | 高 |
| PostgreSQL | 接続数 | > 100 | 中 |
| PostgreSQL | クエリレイテンシ | > 100ms | 中 |
| Kubernetes | ノードCPU使用率 | > 80% | 中 |
| Kubernetes | ノードメモリ使用率 | > 80% | 中 |
| Kubernetes | ノードディスク使用率 | > 80% | 高 |
| Kubernetes | Pod再起動回数 | > 3/時間 | 高 |

### 3.2 Grafanaダッシュボード

以下のダッシュボードを用意しています：

1. **システム概要ダッシュボード**: 全体的なシステム状態、リクエスト数、エラー率
2. **パフォーマンスダッシュボード**: レスポンス時間分布、スループット、キャッシュヒット率
3. **ランタイムダッシュボード**: 言語別実行時間、エラー率、リソース使用率
4. **インフラストラクチャダッシュボード**: Kubernetesノード状態、ポッド状態

### 3.3 アラート設定

アラートは以下の経路で通知されます：

- **緊急（P1）**: Slack + SMS + メール
- **重要（P2）**: Slack + メール
- **通常（P3）**: Slack

## 4. 障害対応

### 4.1 障害レベル定義

| レベル | 説明 | 対応時間 | 通知先 |
|--------|------|---------|--------|
| P1（緊急） | サービス全体が利用不可 | 即時（24/7） | 全チーム + 経営層 |
| P2（重大） | 主要機能が利用不可または著しく低下 | 2時間以内（営業時間） | 運用チーム + 開発リード |
| P3（軽微） | 一部機能に影響、回避策あり | 24時間以内（営業時間） | 運用チーム |
| P4（計画的） | 影響なし、計画的対応可能 | 計画的 | 担当者のみ |

### 4.2 障害対応フロー

1. **検知**: 監視システムまたは報告による障害検知
2. **トリアージ**: 障害の影響範囲と重要度の評価
3. **エスカレーション**: 必要に応じて適切なチームへエスカレーション
4. **初期対応**: サービス復旧のための応急処置
5. **根本原因分析**: 障害の根本原因の特定
6. **恒久対策**: 再発防止のための対策実施
7. **報告**: 障害報告書の作成と共有

### 4.3 一般的な障害と対応手順

#### 4.3.1 Envoy Proxy障害

**症状**: APIリクエストがタイムアウトまたはエラー応答

**対応手順**:

1. Envoy Proxyのログ確認
   ```bash
   kubectl logs -n ingress -l app=envoy --tail=100
   ```

2. Envoy Proxyのステータス確認
   ```bash
   kubectl get pods -n ingress -l app=envoy
   ```

3. 必要に応じて再起動
   ```bash
   kubectl rollout restart deployment envoy -n ingress
   ```

#### 4.3.2 Rustコントローラ障害

**症状**: リクエスト処理エラーまたは遅延

**対応手順**:

1. コントローラのログ確認
   ```bash
   kubectl logs -n controller -l app=rust-controller --tail=100
   ```

2. コントローラのステータス確認
   ```bash
   kubectl get pods -n controller -l app=rust-controller
   ```

3. 必要に応じて再起動
   ```bash
   kubectl rollout restart deployment rust-controller -n controller
   ```

## 5. バックアップと復元

### 5.1 バックアップ戦略

| データ種別 | バックアップ頻度 | 保持期間 | 方式 |
|-----------|----------------|---------|------|
| PostgreSQLデータ | 日次（フル） + 1時間ごと（増分） | 30日 | 論理バックアップ（pg_dump） |
| Redisデータ | 6時間ごと | 7日 | RDBスナップショット |
| 設定データ | 変更時 | 90日 | GitOps（バージョン管理） |
| ログデータ | リアルタイム | 90日 | Elasticsearchスナップショット |

### 5.2 バックアップ手順

#### 5.2.1 PostgreSQLバックアップ

**自動バックアップ（CronJob）**:

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-backup
  namespace: data-services
spec:
  schedule: "0 1 * * *"  # 毎日午前1時
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: postgres-backup
            image: bitnami/postgresql:latest
            command:
            - /bin/sh
            - -c
            - |
              pg_dump -h postgresql.data-services.svc.cluster.local -U lambda_user -d lambda_logs -F c -f /backups/lambda_logs_$(date +%Y%m%d).dump
            env:
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: postgresql
                  key: password
            volumeMounts:
            - name: backup-volume
              mountPath: /backups
          volumes:
          - name: backup-volume
            persistentVolumeClaim:
              claimName: postgres-backup-pvc
          restartPolicy: OnFailure
```

### 5.3 復元手順

```bash
# バックアップファイルをポッドにコピー
kubectl cp ./lambda_logs_backup.dump data-services/postgresql-0:/tmp/

# 復元実行
kubectl exec -it -n data-services postgresql-0 -- bash
pg_restore -U lambda_user -d lambda_logs -c /tmp/lambda_logs_backup.dump
```

## 6. キャパシティ管理

### 6.1 リソース監視

以下のリソースを定期的に監視し、キャパシティプランニングを行います：

- **CPU使用率**: 平均と最大値
- **メモリ使用率**: 平均と最大値
- **ディスク使用率**: 使用量と増加率
- **ネットワークトラフィック**: 帯域使用率
- **リクエスト数**: 時間帯別の平均と最大値
- **データベース容量**: 使用量と増加率
- **キャッシュヒット率**: 効率性指標

### 6.2 スケーリングポリシー

#### 6.2.1 水平スケーリング（HPA）

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rust-controller-hpa
  namespace: controller
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rust-controller
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

## 7. セキュリティ運用

### 7.1 認証・認可管理

APIキーのライフサイクル管理：

- **発行**: 承認フローに基づく発行
- **更新**: 90日ごとの自動更新通知
- **失効**: 未使用90日経過後の自動失効
- **監査**: 月次使用状況レビュー

### 7.2 脆弱性管理

週次で以下のスキャンを実施：

- **コンテナイメージスキャン**: Trivyによるスキャン
- **Kubernetes設定スキャン**: kube-benchによるスキャン
- **依存ライブラリスキャン**: OWASP Dependency Checkによるスキャン

### 7.3 監査ログ管理

| ログ種別 | 保持期間 | アーカイブ期間 |
|---------|---------|--------------|
| アプリケーションログ | 30日 | 1年 |
| 監査ログ | 90日 | 7年 |
| アクセスログ | 30日 | 1年 |
| セキュリティイベント | 90日 | 7年 |

## 8. 定期メンテナンス

### 8.1 メンテナンススケジュール

| 作業 | 頻度 | 所要時間 | 影響 |
|------|------|---------|------|
| Kubernetesアップデート | 四半期ごと | 4時間 | 一部サービス中断 |
| データベースメンテナンス | 月次 | 2時間 | 読み取り専用モード |
| バックアップ検証 | 四半期ごと | 4時間 | 影響なし |
| セキュリティパッチ適用 | 月次 | 2時間 | ローリング更新（影響最小） |
| パフォーマンスチューニング | 四半期ごと | 4時間 | 一部遅延の可能性 |

### 8.2 メンテナンス手順

#### 8.2.1 Kubernetesアップデート

```bash
# 現在のバージョン確認
kubectl version

# ノードのドレイン
kubectl drain <node-name> --ignore-daemonsets

# ノードのアップデート
# クラウドプロバイダーの手順に従う

# ノードの復帰
kubectl uncordon <node-name>
```

## 9. 新機能デプロイ

### 9.1 デプロイフロー

1. **開発環境テスト**: 機能開発後の初期テスト
2. **コードレビュー**: 最低2名の承認
3. **テスト環境デプロイ**: 自動テスト実行
4. **ステージング環境デプロイ**: 手動テストと検証
5. **本番環境デプロイ**: カナリアリリースまたはブルー/グリーンデプロイ
6. **モニタリング**: デプロイ後の監視強化
7. **ロールバック準備**: 問題発生時の迅速な対応

### 9.2 カナリアリリース手順

```yaml
apiVersion: networking.istio.io/v1alpha3
kind: VirtualService
metadata:
  name: rust-controller
  namespace: controller
spec:
  hosts:
  - rust-controller
  http:
  - route:
    - destination:
        host: rust-controller
        subset: v1
      weight: 90
    - destination:
        host: rust-controller
        subset: v2
      weight: 10
```

## 10. パフォーマンスチューニング

### 10.1 パフォーマンス監視

定期的に以下のメトリクスを分析し、パフォーマンス最適化を行います：

- **レスポンス時間**: 平均、95パーセンタイル、99パーセンタイル
- **スループット**: 1秒あたりのリクエスト数
- **エラー率**: リクエスト失敗の割合
- **リソース使用率**: CPU、メモリ、ディスク、ネットワーク
- **キャッシュヒット率**: Redisキャッシュの効率
- **データベースパフォーマンス**: クエリ実行時間、接続数

### 10.2 チューニングポイント

#### 10.2.1 Rustコントローラチューニング

```bash
# 現在の設定確認
kubectl get configmap -n controller controller-env -o yaml

# 設定更新
cat > controller-env-update.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: controller-env
  namespace: controller
data:
  CACHE_TTL: "7200"  # 2時間に延長
  REQUEST_TIMEOUT: "3000"  # 3秒に短縮
  WORKER_THREADS: "16"  # ワーカースレッド数調整
EOF

kubectl apply -f controller-env-update.yaml
```

## 11. トラブルシューティングガイド

### 11.1 一般的な問題と解決策

| 問題 | 症状 | 確認事項 | 解決策 |
|------|------|---------|--------|
| APIレスポンス遅延 | レスポンス時間 > 100ms | - Rustコントローラのログ<br>- リソース使用率<br>- データベース接続 | - スケールアウト<br>- キャッシュ設定見直し<br>- クエリ最適化 |
| 高エラー率 | エラーレート > 1% | - エラーログ<br>- 特定のパターン<br>- 外部依存関係 | - 根本原因特定<br>- 一時的な回避策<br>- 恒久対策 |
| メモリリーク | メモリ使用率の継続的増加 | - コンテナメモリ使用量<br>- ヒープダンプ<br>- GCログ | - コンテナ再起動<br>- メモリリーク修正<br>- リソース制限調整 |

### 11.2 ログ分析ガイド

#### 11.2.1 重要なログパターン

| コンポーネント | ログパターン | 意味 | 対応 |
|--------------|------------|------|------|
| Rustコントローラ | `ERROR: Connection refused` | Redisまたは他のサービスへの接続失敗 | 接続先サービスの状態確認 |
| Rustコントローラ | `WARN: High latency detected` | 処理遅延の検出 | パフォーマンス調査 |
| OpenFaaS | `Error: function not found` | 関数が見つからない | 関数デプロイ状態確認 |
| Envoy | `upstream connect error` | バックエンドサービスへの接続エラー | バックエンドサービス確認 |

## 12. 連絡先とエスカレーションパス

### 12.1 担当者一覧

| 役割 | 担当者 | 連絡先 | 対応時間 |
|------|--------|--------|---------|
| 運用リード | 山田太郎 | yamada@example.com<br>090-XXXX-XXXX | 平日 9:00-18:00 |
| インフラ担当 | 鈴木一郎 | suzuki@example.com<br>090-XXXX-XXXX | 平日 9:00-18:00 |
| 開発リード | 佐藤花子 | sato@example.com<br>090-XXXX-XXXX | 平日 9:00-18:00 |
| オンコール担当 | 当番制 | oncall@example.com<br>090-XXXX-XXXX | 24/7 |

### 12.2 エスカレーションパス

| 障害レベル | 一次対応 | 二次対応 | 最終エスカレーション |
|-----------|---------|---------|-------------------|
| P1（緊急） | オンコール担当 | 運用リード + 開発リード | CTO |
| P2（重大） | オンコール担当 | 運用リード | 開発リード |
| P3（軽微） | オンコール担当 | 運用担当 | 運用リード |
| P4（計画的） | 運用担当 | - | - |
