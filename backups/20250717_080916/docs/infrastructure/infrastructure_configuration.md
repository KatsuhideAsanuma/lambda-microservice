# インフラストラクチャ構成図と設計書

## 1. 概要

lambda-microservice基盤のインフラストラクチャ構成を詳細に定義します。このドキュメントは、Kubernetes上にデプロイされるコンポーネントの構成、ネットワーク設計、スケーリング戦略、および高可用性設計を含みます。

## 2. 全体構成図

```
                                    ┌─────────────────────────────────────────────────────┐
                                    │               Kubernetes Cluster                     │
                                    │                                                     │
┌─────────┐     ┌─────────┐        │  ┌─────────┐    ┌─────────────────────────────┐     │
│         │     │         │        │  │         │    │  OpenFaaS Namespace         │     │
│ Internet│────▶│  Envoy  │───────▶│  │ OpenFaaS│    │                             │     │
│         │     │ Proxy   │        │  │ Gateway │    │  ┌─────────┐  ┌─────────┐   │     │
└─────────┘     └─────────┘        │  │         │    │  │ Node.js │  │ Python  │   │     │
                                    │  └────┬────┘    │  │ Runtime │  │ Runtime │   │     │
                                    │       │         │  └─────────┘  └─────────┘   │     │
                                    │       │         │                             │     │
                                    │       │         │       ┌─────────┐           │     │
                                    │       │         │       │  Rust   │           │     │
                                    │       │         │       │ Runtime │           │     │
                                    │       │         │       └─────────┘           │     │
                                    │       │         └─────────────────────────────┘     │
                                    │       │                                             │
                                    │       ▼                                             │
                                    │  ┌─────────┐    ┌─────────────────────────────┐     │
                                    │  │         │    │  Monitoring Namespace       │     │
                                    │  │  Rust   │    │                             │     │
                                    │  │Controller    │  ┌─────────┐  ┌─────────┐   │     │
                                    │  │         │    │  │Prometheus│  │ Grafana │   │     │
                                    │  └────┬────┘    │  └─────────┘  └─────────┘   │     │
                                    │       │         │                             │     │
                                    │       │         │  ┌───────────────────┐      │     │
                                    │       │         │  │   Elastic Stack   │      │     │
                                    │       │         │  └───────────────────┘      │     │
                                    │       │         └─────────────────────────────┘     │
                                    │       │                                             │
                                    │       ▼         ┌─────────────────────────────┐     │
                                    │  ┌─────────┐    │  Data Services Namespace    │     │
                                    │  │         │    │                             │     │
                                    │  │  Redis  │◀───│──┐ ┌─────────┐              │     │
                                    │  │ Cluster │    │  └─│ Redis   │              │     │
                                    │  │         │    │    │ Sentinel│              │     │
                                    │  └─────────┘    │    └─────────┘              │     │
                                    │       │         │                             │     │
                                    │       │         │    ┌─────────┐              │     │
                                    │       └────────▶│────│PostgreSQL              │     │
                                    │                 │    │ Cluster │              │     │
                                    │                 │    └─────────┘              │     │
                                    │                 └─────────────────────────────┘     │
                                    └─────────────────────────────────────────────────────┘
```

## 3. コンポーネント詳細

### 3.1 Kubernetes クラスター

**仕様**:
- クラウドプロバイダー: AWS EKS または Google GKE
- Kubernetesバージョン: 1.25以上
- ノードタイプ:
  - コントロールプレーン: 3ノード（高可用性）
  - ワーカーノード: 自動スケーリング（最小3、最大20）
- ノードスペック:
  - CPU: 4 vCPU以上
  - メモリ: 16GB以上
  - ストレージ: 100GB SSD

**Namespaces**:
- `ingress`: Envoy Proxy
- `openfaas`: OpenFaaS関連コンポーネント
- `openfaas-fn`: OpenFaaS関数
- `controller`: Rustコントローラ
- `monitoring`: Prometheus, Grafana, Elastic Stack
- `data-services`: Redis, PostgreSQL

### 3.2 Envoy Proxy

**デプロイメント**:
- レプリカ数: 3（高可用性）
- リソース制限:
  - CPU: 2 vCPU
  - メモリ: 4GB

**設定**:
- TLS終端
- ヘッダーベースルーティング
- レートリミット
- リトライポリシー
- サーキットブレーカー

**サービス**:
- タイプ: LoadBalancer
- ポート: 443（HTTPS）、80（HTTP→HTTPSリダイレクト）

### 3.3 OpenFaaS

**コンポーネント**:
- Gateway
- Queue Worker
- Prometheus (OpenFaaS専用)
- NATS

**デプロイメント**:
- Gateway レプリカ数: 3
- Queue Worker レプリカ数: 2

**設定**:
- 自動スケーリング（HPA）
- 関数タイムアウト: 30秒
- 関数メモリ制限: 256MB（デフォルト）

### 3.4 Rustコントローラ

**デプロイメント**:
- レプリカ数: 3（高可用性）
- リソース制限:
  - CPU: 2 vCPU
  - メモリ: 4GB

**設定**:
- ヘルスチェック
- リードネスプローブ
- ライブネスプローブ
- 水平自動スケーリング（HPA）

**サービス**:
- タイプ: ClusterIP
- ポート: 8080（API）、9090（メトリクス）

### 3.5 ランタイムコンテナ

**共通設定**:
- 自動スケーリング（HPA）
- リソース制限（デフォルト）:
  - CPU: 1 vCPU
  - メモリ: 2GB
- タイムアウト: 30秒

**Node.js ランタイム**:
- バージョン: Node.js 18 LTS
- ベースイメージ: node:18-alpine

**Python ランタイム**:
- バージョン: Python 3.10
- ベースイメージ: python:3.10-slim

**Rust ランタイム**:
- バージョン: Rust 1.67
- ベースイメージ: rust:1.67-slim

### 3.6 Redis

**デプロイメント**:
- アーキテクチャ: Redis Sentinel
- マスターノード: 1
- レプリカノード: 2
- Sentinelノード: 3

**リソース制限**:
- CPU: 2 vCPU
- メモリ: 8GB

**永続化**:
- PersistentVolumeClaim: 20GB

**設定**:
- maxmemory: 6GB
- maxmemory-policy: allkeys-lru
- AOF永続化: yes

### 3.7 PostgreSQL

**デプロイメント**:
- アーキテクチャ: Primary-Standby
- プライマリノード: 1
- スタンバイノード: 2

**リソース制限**:
- CPU: 4 vCPU
- メモリ: 16GB

**永続化**:
- PersistentVolumeClaim: 100GB

**設定**:
- WAL設定: 最適化済み
- 接続プール: PgBouncer
- バックアップ: 日次スナップショット

### 3.8 監視・ログ収集

**Prometheus**:
- データ保持期間: 15日
- スクレイプ間隔: 15秒
- アラートマネージャー統合

**Grafana**:
- データソース: Prometheus, PostgreSQL
- 事前設定ダッシュボード:
  - システム概要
  - ランタイムパフォーマンス
  - リクエスト統計
  - エラー率

**Elastic Stack**:
- Elasticsearch: 3ノードクラスター
- Kibana: 1レプリカ
- Filebeat: DaemonSet
- Logstash: 2レプリカ

## 4. ネットワーク設計

### 4.1 サービスメッシュ

**Istio**を使用したサービスメッシュ実装:
- 相互TLS（mTLS）
- トラフィック管理
- 可視化（Kiali）

### 4.2 ネットワークポリシー

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: restrict-access
spec:
  podSelector:
    matchLabels:
      app: rust-controller
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress
    - namespaceSelector:
        matchLabels:
          name: openfaas
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: openfaas-fn
    - namespaceSelector:
        matchLabels:
          name: data-services
```

### 4.3 サービスディスカバリー

Kubernetes内蔵のサービスディスカバリーを使用:
- サービス名: `{component}.{namespace}.svc.cluster.local`
- 例: `redis-master.data-services.svc.cluster.local`

## 5. スケーリング戦略

### 5.1 水平自動スケーリング（HPA）

**Rustコントローラ**:
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rust-controller-hpa
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
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

**ランタイムコンテナ**:
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: nodejs-runtime-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: nodejs-runtime
  minReplicas: 2
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
    scaleUp:
      stabilizationWindowSeconds: 60
```

### 5.2 垂直自動スケーリング（VPA）

**オプション機能**として、Vertical Pod Autoscalerを使用:
- 初期段階: リソース使用量の監視モード
- 安定段階: 自動リソース調整モード

## 6. 高可用性設計

### 6.1 ゾーン分散

- マルチAZ（Availability Zone）デプロイメント
- ノードアフィニティルールによる分散配置

```yaml
affinity:
  nodeAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      nodeSelectorTerms:
      - matchExpressions:
        - key: topology.kubernetes.io/zone
          operator: In
          values:
          - us-east-1a
          - us-east-1b
          - us-east-1c
```

### 6.2 Pod分散

```yaml
topologySpreadConstraints:
- maxSkew: 1
  topologyKey: topology.kubernetes.io/zone
  whenUnsatisfiable: DoNotSchedule
  labelSelector:
    matchLabels:
      app: rust-controller
```

### 6.3 障害復旧

- StatefulSetによるステートフル管理
- PodDisruptionBudgetによる計画的メンテナンス制御
- 自動バックアップと復元手順

## 7. セキュリティ設計

### 7.1 ポッドセキュリティポリシー

```yaml
apiVersion: policy/v1beta1
kind: PodSecurityPolicy
metadata:
  name: restricted
spec:
  privileged: false
  allowPrivilegeEscalation: false
  requiredDropCapabilities:
    - ALL
  volumes:
    - 'configMap'
    - 'emptyDir'
    - 'projected'
    - 'secret'
    - 'downwardAPI'
    - 'persistentVolumeClaim'
  hostNetwork: false
  hostIPC: false
  hostPID: false
  runAsUser:
    rule: 'MustRunAsNonRoot'
  seLinux:
    rule: 'RunAsAny'
  supplementalGroups:
    rule: 'MustRunAs'
    ranges:
      - min: 1
        max: 65535
  fsGroup:
    rule: 'MustRunAs'
    ranges:
      - min: 1
        max: 65535
  readOnlyRootFilesystem: true
```

### 7.2 シークレット管理

- Kubernetes Secretsを使用
- 外部シークレット管理（オプション）:
  - AWS Secrets Manager
  - HashiCorp Vault

### 7.3 ネットワークセキュリティ

- Istio mTLS
- ネットワークポリシー
- Envoyによる境界保護

## 8. リソース要件

| コンポーネント | 最小ノード数 | CPU (各) | メモリ (各) | ストレージ (各) |
|--------------|------------|---------|-----------|--------------|
| Envoy Proxy | 3 | 2 vCPU | 4 GB | - |
| OpenFaaS Gateway | 3 | 2 vCPU | 4 GB | - |
| Rustコントローラ | 3 | 2 vCPU | 4 GB | - |
| Node.jsランタイム | 2-20 | 1 vCPU | 2 GB | - |
| Pythonランタイム | 2-20 | 1 vCPU | 2 GB | - |
| Rustランタイム | 2-20 | 1 vCPU | 2 GB | - |
| Redis | 3 | 2 vCPU | 8 GB | 20 GB |
| PostgreSQL | 3 | 4 vCPU | 16 GB | 100 GB |
| Prometheus | 1 | 2 vCPU | 8 GB | 50 GB |
| Grafana | 1 | 1 vCPU | 2 GB | 10 GB |
| Elasticsearch | 3 | 4 vCPU | 16 GB | 100 GB |
| Kibana | 1 | 1 vCPU | 2 GB | - |
| Logstash | 2 | 2 vCPU | 4 GB | - |

**合計最小リソース要件**:
- CPU: 約70 vCPU
- メモリ: 約200 GB
- ストレージ: 約300 GB

## 9. 推奨クラウド構成

### 9.1 AWS EKS

- リージョン: us-east-1（または最寄りリージョン）
- ノードタイプ: m5.2xlarge（8 vCPU, 32 GB RAM）
- ノード数: 最小6、最大20
- ストレージ: gp3 SSD
- ネットワーク: VPC、プライベートサブネット
- ロードバランサー: ALB（Application Load Balancer）

### 9.2 Google GKE

- リージョン: us-central1（または最寄りリージョン）
- ノードタイプ: e2-standard-8（8 vCPU, 32 GB RAM）
- ノード数: 最小6、最大20
- ストレージ: SSD Persistent Disk
- ネットワーク: VPC、プライベートサブネット
- ロードバランサー: Cloud Load Balancing

## 10. 障害シナリオと対応

| シナリオ | 影響 | 自動対応 | 手動対応 |
|---------|------|---------|---------|
| ノード障害 | 影響ノード上のPod停止 | Kubernetes自動再スケジュール | 必要に応じてノード置換 |
| AZ障害 | 該当AZのリソース利用不可 | 他AZへの自動フェイルオーバー | クラスター容量確認・調整 |
| Redisマスター障害 | 一時的な書き込み不可 | Sentinelによる自動フェイルオーバー | 新レプリカ追加 |
| PostgreSQLプライマリ障害 | 一時的な書き込み不可 | 自動フェイルオーバー | 新スタンバイ追加 |
| Envoy障害 | 一部リクエスト失敗 | 正常ノードへの自動ルーティング | 設定確認・修正 |
| コントローラ障害 | 一部リクエスト処理遅延 | 正常ノードへの自動ルーティング | ログ確認・デバッグ |
