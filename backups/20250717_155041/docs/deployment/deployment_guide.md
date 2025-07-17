# デプロイメントガイド

## 1. 概要

このガイドでは、lambda-microservice基盤の環境構築とデプロイメントの手順を詳細に説明します。本システムは、Kubernetes上にOpenFaaS、Envoy Proxy、Rustコントローラ、各種ランタイムコンテナ、およびデータサービス（Redis、PostgreSQL）をデプロイします。

## 2. 前提条件

### 2.1 必要なツール

| ツール | バージョン | 用途 |
|-------|-----------|------|
| kubectl | 1.25+ | Kubernetesクラスター管理 |
| helm | 3.8+ | Kubernetesパッケージマネージャー |
| docker | 20.10+ | コンテナイメージビルド |
| faas-cli | 0.14+ | OpenFaaS CLI |
| terraform | 1.3+ | インフラストラクチャ管理（オプション） |
| aws-cli / gcloud | 最新 | クラウドプロバイダーCLI |
| git | 2.30+ | ソースコード管理 |
| cargo | 1.67+ | Rustビルドツール |
| node | 18+ | Node.jsランタイム開発 |
| python | 3.10+ | Pythonランタイム開発 |

### 2.2 アクセス権限

- Kubernetesクラスター管理者権限
- コンテナレジストリ（ECR/GCR）へのプッシュ権限
- クラウドリソース作成権限（EKS/GKE）

## 3. 環境構築

### 3.1 Kubernetesクラスター作成

#### AWS EKS

```bash
# EKSクラスター作成
eksctl create cluster \
  --name lambda-microservice \
  --region us-east-1 \
  --version 1.25 \
  --nodegroup-name standard-nodes \
  --node-type m5.2xlarge \
  --nodes 6 \
  --nodes-min 3 \
  --nodes-max 20 \
  --with-oidc \
  --ssh-access \
  --ssh-public-key my-key \
  --managed

# クラスター認証情報の取得
aws eks update-kubeconfig --name lambda-microservice --region us-east-1
```

#### Google GKE

```bash
# GKEクラスター作成
gcloud container clusters create lambda-microservice \
  --region us-central1 \
  --num-nodes 2 \
  --machine-type e2-standard-8 \
  --min-nodes 1 \
  --max-nodes 20 \
  --enable-autoscaling \
  --release-channel regular

# クラスター認証情報の取得
gcloud container clusters get-credentials lambda-microservice --region us-central1
```

### 3.2 Namespaces作成

```bash
# 必要なNamespaces作成
kubectl create namespace ingress
kubectl create namespace openfaas
kubectl create namespace openfaas-fn
kubectl create namespace controller
kubectl create namespace monitoring
kubectl create namespace data-services

# ラベル付与（サービスメッシュ用）
kubectl label namespace ingress istio-injection=enabled
kubectl label namespace openfaas istio-injection=enabled
kubectl label namespace openfaas-fn istio-injection=enabled
kubectl label namespace controller istio-injection=enabled
```

### 3.3 Istioインストール（オプション）

```bash
# Istioインストール
istioctl install --set profile=default -y

# Kiali, Prometheus, Grafanaインストール
kubectl apply -f https://raw.githubusercontent.com/istio/istio/release-1.17/samples/addons/kiali.yaml
kubectl apply -f https://raw.githubusercontent.com/istio/istio/release-1.17/samples/addons/prometheus.yaml
kubectl apply -f https://raw.githubusercontent.com/istio/istio/release-1.17/samples/addons/grafana.yaml
```

### 3.4 OpenFaaSインストール

```bash
# OpenFaaS Helmリポジトリ追加
helm repo add openfaas https://openfaas.github.io/faas-netes/
helm repo update

# OpenFaaSインストール
helm upgrade --install openfaas openfaas/openfaas \
  --namespace openfaas \
  --set functionNamespace=openfaas-fn \
  --set serviceType=ClusterIP \
  --set gateway.replicas=3 \
  --set queueWorker.replicas=2 \
  --set operator.create=true \
  --set faasnetes.imagePullPolicy=Always \
  --set gateway.directFunctions=false \
  --set gateway.readTimeout=60s \
  --set gateway.writeTimeout=60s \
  --set gateway.upstreamTimeout=55s \
  --set queueWorker.ackWait=60s

# OpenFaaS CLI認証情報取得
PASSWORD=$(kubectl -n openfaas get secret basic-auth -o jsonpath="{.data.basic-auth-password}" | base64 --decode)
echo $PASSWORD | faas-cli login --username admin --password-stdin
```

### 3.5 Envoy Proxyインストール

```bash
# Envoy Proxy設定ファイル作成
cat > envoy-config.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: envoy-config
  namespace: ingress
data:
  envoy.yaml: |
    admin:
      access_log_path: /tmp/admin_access.log
      address:
        socket_address:
          protocol: TCP
          address: 0.0.0.0
          port_value: 9901
    static_resources:
      listeners:
      - name: listener_0
        address:
          socket_address:
            address: 0.0.0.0
            port_value: 8080
        filter_chains:
        - filters:
          - name: envoy.filters.network.http_connection_manager
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager
              stat_prefix: ingress_http
              route_config:
                name: local_route
                virtual_hosts:
                - name: local_service
                  domains: ["*"]
                  routes:
                  - match:
                      prefix: "/"
                    route:
                      cluster: openfaas_gateway
                      timeout: 60s
              http_filters:
              - name: envoy.filters.http.router
                typed_config:
                  "@type": type.googleapis.com/envoy.extensions.filters.http.router.v3.Router
      clusters:
      - name: openfaas_gateway
        connect_timeout: 0.25s
        type: STRICT_DNS
        lb_policy: ROUND_ROBIN
        load_assignment:
          cluster_name: openfaas_gateway
          endpoints:
          - lb_endpoints:
            - endpoint:
                address:
                  socket_address:
                    address: gateway.openfaas.svc.cluster.local
                    port_value: 8080
EOF

# Envoy Proxy設定適用
kubectl apply -f envoy-config.yaml

# Envoy Proxyデプロイメント作成
cat > envoy-deployment.yaml << EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: envoy
  namespace: ingress
spec:
  replicas: 3
  selector:
    matchLabels:
      app: envoy
  template:
    metadata:
      labels:
        app: envoy
    spec:
      containers:
      - name: envoy
        image: envoyproxy/envoy:v1.25-latest
        resources:
          limits:
            cpu: 2000m
            memory: 4Gi
          requests:
            cpu: 500m
            memory: 1Gi
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9901
          name: admin
        volumeMounts:
        - name: envoy-config
          mountPath: /etc/envoy
        command: ["envoy"]
        args:
          - "--config-path /etc/envoy/envoy.yaml"
          - "--service-cluster envoy"
          - "--service-node envoy"
          - "--log-level info"
      volumes:
      - name: envoy-config
        configMap:
          name: envoy-config
EOF

# Envoy Proxyサービス作成
cat > envoy-service.yaml << EOF
apiVersion: v1
kind: Service
metadata:
  name: envoy
  namespace: ingress
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9901
    targetPort: 9901
    protocol: TCP
    name: admin
  selector:
    app: envoy
EOF

# Envoy Proxyデプロイ
kubectl apply -f envoy-deployment.yaml
kubectl apply -f envoy-service.yaml
```

### 3.6 データサービスインストール

#### Redis

```bash
# Redis Helmリポジトリ追加
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo update

# Redisインストール（Sentinelモード）
helm install redis bitnami/redis \
  --namespace data-services \
  --set sentinel.enabled=true \
  --set sentinel.masterSet=mymaster \
  --set sentinel.downAfterMilliseconds=5000 \
  --set sentinel.failoverTimeout=10000 \
  --set sentinel.parallelSyncs=1 \
  --set master.persistence.size=20Gi \
  --set replica.replicaCount=2 \
  --set replica.persistence.size=20Gi \
  --set metrics.enabled=true
```

#### PostgreSQL

```bash
# PostgreSQLインストール
helm install postgresql bitnami/postgresql \
  --namespace data-services \
  --set global.postgresql.auth.postgresPassword=postgres_password \
  --set global.postgresql.auth.username=lambda_user \
  --set global.postgresql.auth.password=lambda_password \
  --set global.postgresql.auth.database=lambda_logs \
  --set primary.initdb.scriptsConfigMap=postgresql-initdb \
  --set primary.persistence.size=100Gi \
  --set readReplicas.replicaCount=2 \
  --set readReplicas.persistence.size=100Gi \
  --set metrics.enabled=true

# PostgreSQL初期化スクリプト作成
cat > postgresql-initdb.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: postgresql-initdb
  namespace: data-services
data:
  init.sql: |
    CREATE DATABASE lambda_meta;
    
    \c lambda_logs;
    
    CREATE SCHEMA IF NOT EXISTS public;
    CREATE SCHEMA IF NOT EXISTS analytics;
    
    \c lambda_meta;
    
    CREATE SCHEMA IF NOT EXISTS meta;
EOF

kubectl apply -f postgresql-initdb.yaml
```

### 3.7 監視システムインストール

#### Prometheus & Grafana

```bash
# Prometheus Operatorインストール
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --set prometheus.prometheusSpec.serviceMonitorSelectorNilUsesHelmValues=false \
  --set prometheus.prometheusSpec.podMonitorSelectorNilUsesHelmValues=false \
  --set grafana.enabled=true \
  --set grafana.adminPassword=admin_password \
  --set grafana.persistence.enabled=true \
  --set grafana.persistence.size=10Gi
```

#### Elastic Stack

```bash
# Elastic Stackインストール
helm repo add elastic https://helm.elastic.co
helm repo update

# Elasticsearchインストール
helm install elasticsearch elastic/elasticsearch \
  --namespace monitoring \
  --set replicas=3 \
  --set minimumMasterNodes=2 \
  --set volumeClaimTemplate.resources.requests.storage=100Gi

# Kibanaインストール
helm install kibana elastic/kibana \
  --namespace monitoring \
  --set elasticsearchHosts=http://elasticsearch-master:9200

# Filebeatインストール
helm install filebeat elastic/filebeat \
  --namespace monitoring \
  --set daemonset.enabled=true \
  --set filebeatConfig.filebeat.yml="filebeat.inputs:
  - type: container
    paths:
      - /var/log/containers/*.log
    processors:
      - add_kubernetes_metadata:
          host: \${NODE_NAME}
          matchers:
          - logs_path:
              logs_path: \"/var/log/containers/\"
output.elasticsearch:
  host: '\${ELASTICSEARCH_HOST:elasticsearch-master}:\${ELASTICSEARCH_PORT:9200}'
  username: \${ELASTICSEARCH_USERNAME}
  password: \${ELASTICSEARCH_PASSWORD}"
```

## 4. アプリケーションのビルドとデプロイ

### 4.1 コンテナレジストリ設定

#### AWS ECR

```bash
# ECRリポジトリ作成
aws ecr create-repository --repository-name lambda-microservice/rust-controller
aws ecr create-repository --repository-name lambda-microservice/nodejs-runtime
aws ecr create-repository --repository-name lambda-microservice/python-runtime
aws ecr create-repository --repository-name lambda-microservice/rust-runtime

# ECRログイン
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <AWS_ACCOUNT_ID>.dkr.ecr.us-east-1.amazonaws.com
```

#### Google GCR

```bash
# GCRログイン
gcloud auth configure-docker

# リポジトリは自動作成されるため、明示的な作成は不要
```

### 4.2 Rustコントローラのビルドとデプロイ

```bash
# ソースコードディレクトリに移動
cd controller

# Dockerイメージビルド
docker build -t lambda-microservice/rust-controller:latest .

# イメージタグ付け（AWS ECR）
docker tag lambda-microservice/rust-controller:latest <AWS_ACCOUNT_ID>.dkr.ecr.us-east-1.amazonaws.com/lambda-microservice/rust-controller:latest

# イメージプッシュ（AWS ECR）
docker push <AWS_ACCOUNT_ID>.dkr.ecr.us-east-1.amazonaws.com/lambda-microservice/rust-controller:latest

# または、イメージタグ付け（Google GCR）
docker tag lambda-microservice/rust-controller:latest gcr.io/<GCP_PROJECT_ID>/lambda-microservice/rust-controller:latest

# イメージプッシュ（Google GCR）
docker push gcr.io/<GCP_PROJECT_ID>/lambda-microservice/rust-controller:latest

# Kubernetesデプロイメント作成
cat > controller-deployment.yaml << EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rust-controller
  namespace: controller
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rust-controller
  template:
    metadata:
      labels:
        app: rust-controller
    spec:
      containers:
      - name: rust-controller
        image: <REGISTRY_URL>/lambda-microservice/rust-controller:latest
        imagePullPolicy: Always
        resources:
          limits:
            cpu: 2000m
            memory: 4Gi
          requests:
            cpu: 500m
            memory: 1Gi
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: REDIS_URL
          value: redis://redis-master.data-services.svc.cluster.local:6379
        - name: POSTGRES_URL
          valueFrom:
            secretKeyRef:
              name: controller-secrets
              key: postgres-url
        - name: LOG_LEVEL
          value: info
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
EOF

# シークレット作成
kubectl create secret generic controller-secrets \
  --namespace controller \
  --from-literal=postgres-url="postgres://lambda_user:lambda_password@postgresql.data-services.svc.cluster.local:5432/lambda_logs"

# デプロイメント適用
kubectl apply -f controller-deployment.yaml

# サービス作成
cat > controller-service.yaml << EOF
apiVersion: v1
kind: Service
metadata:
  name: rust-controller
  namespace: controller
spec:
  type: ClusterIP
  ports:
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
  selector:
    app: rust-controller
EOF

kubectl apply -f controller-service.yaml

# サービスモニター作成（Prometheus用）
cat > controller-servicemonitor.yaml << EOF
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rust-controller
  namespace: monitoring
spec:
  selector:
    matchLabels:
      app: rust-controller
  namespaceSelector:
    matchNames:
      - controller
  endpoints:
  - port: metrics
    interval: 15s
EOF

kubectl apply -f controller-servicemonitor.yaml
```

### 4.3 ランタイムコンテナのビルドとデプロイ

#### Node.js ランタイム

```bash
# OpenFaaS Node.jsテンプレート作成
faas-cli template pull https://github.com/openfaas/templates
faas-cli new --lang node18 nodejs-runtime

# カスタマイズ
cd nodejs-runtime
# ここでコードを編集

# ビルドとデプロイ
faas-cli build -f nodejs-runtime.yml
faas-cli push -f nodejs-runtime.yml
faas-cli deploy -f nodejs-runtime.yml
```

#### Python ランタイム

```bash
# OpenFaaS Pythonテンプレート作成
faas-cli new --lang python3 python-runtime

# カスタマイズ
cd python-runtime
# ここでコードを編集

# ビルドとデプロイ
faas-cli build -f python-runtime.yml
faas-cli push -f python-runtime.yml
faas-cli deploy -f python-runtime.yml
```

#### Rust ランタイム

```bash
# OpenFaaS Rustテンプレート作成
faas-cli template pull https://github.com/openfaas-incubator/rust-http-template
faas-cli new --lang rust-http rust-runtime

# カスタマイズ
cd rust-runtime
# ここでコードを編集

# ビルドとデプロイ
faas-cli build -f rust-runtime.yml
faas-cli push -f rust-runtime.yml
faas-cli deploy -f rust-runtime.yml
```

### 4.4 OpenFaaS Gatewayとの連携設定

```bash
# OpenFaaS Gateway設定更新
cat > gateway-config.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: gateway-config
  namespace: openfaas
data:
  gateway.yml: |
    functions:
      direct_functions: false
      direct_functions_suffix: ""
    http:
      read_timeout: 60s
      write_timeout: 60s
      upstream_timeout: 55s
    scaling:
      max_replica_count: 20
      min_replica_count: 1
      zero_scale: true
      zero_scale_label: "com.openfaas.scale.zero=true"
EOF

kubectl apply -f gateway-config.yaml
kubectl rollout restart deployment gateway -n openfaas
```

## 5. 設定とカスタマイズ

### 5.1 環境変数設定

各コンポーネントの環境変数は、Kubernetes ConfigMapまたはSecretを使用して設定します。

```bash
# コントローラー環境変数設定
cat > controller-env.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: controller-env
  namespace: controller
data:
  CACHE_TTL: "3600"
  REQUEST_TIMEOUT: "5000"
  LOG_LEVEL: "info"
  METRICS_PORT: "9090"
EOF

kubectl apply -f controller-env.yaml
```

### 5.2 スケーリング設定

```bash
# 水平自動スケーリング設定
cat > controller-hpa.yaml << EOF
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
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
EOF

kubectl apply -f controller-hpa.yaml
```

### 5.3 ネットワークポリシー設定

```bash
# ネットワークポリシー設定
cat > network-policy.yaml << EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: restrict-access
  namespace: controller
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
EOF

kubectl apply -f network-policy.yaml
```

## 6. 検証とテスト

### 6.1 デプロイメント検証

```bash
# 各コンポーネントのステータス確認
kubectl get pods -n ingress
kubectl get pods -n openfaas
kubectl get pods -n openfaas-fn
kubectl get pods -n controller
kubectl get pods -n data-services
kubectl get pods -n monitoring

# サービスエンドポイント確認
kubectl get svc -n ingress
```

### 6.2 基本機能テスト

```bash
# Envoy Proxyエンドポイント取得
ENVOY_ENDPOINT=$(kubectl get svc -n ingress envoy -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

# テストリクエスト送信
curl -X POST \
  http://$ENVOY_ENDPOINT/api/v1/execute \
  -H 'Content-Type: application/json' \
  -H 'Language-Title: nodejs-calculator' \
  -d '{
    "params": {
      "operation": "add",
      "values": [1, 2, 3]
    },
    "context": {
      "environment": "test"
    }
  }'
```

### 6.3 監視システム確認

```bash
# Grafanaエンドポイント取得
GRAFANA_ENDPOINT=$(kubectl get svc -n monitoring prometheus-grafana -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

# Kibanaエンドポイント取得
KIBANA_ENDPOINT=$(kubectl get svc -n monitoring kibana-kibana -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

echo "Grafana: http://$GRAFANA_ENDPOINT"
echo "Kibana: http://$KIBANA_ENDPOINT"
```

## 7. 運用とメンテナンス

### 7.1 バックアップ設定

```bash
# PostgreSQLバックアップCronJob設定
cat > postgres-backup.yaml << EOF
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
              pg_dump -h postgresql.data-services.svc.cluster.local -U lambda_user -d lambda_logs -F c -f /backups/lambda_logs_\$(date +%Y%m%d).dump
              pg_dump -h postgresql.data-services.svc.cluster.local -U lambda_user -d lambda_meta -F c -f /backups/lambda_meta_\$(date +%Y%m%d).dump
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
EOF

# バックアップ用PVC作成
cat > postgres-backup-pvc.yaml << EOF
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: postgres-backup-pvc
  namespace: data-services
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
EOF

kubectl apply -f postgres-backup-pvc.yaml
kubectl apply -f postgres-backup.yaml
```

### 7.2 ログローテーション

```bash
# Elasticsearchインデックスライフサイクル設定
cat > elasticsearch-ilm.yaml << EOF
apiVersion: batch/v1
kind: Job
metadata:
  name: setup-elasticsearch-ilm
  namespace: monitoring
spec:
  template:
    spec:
      containers:
      - name: setup-ilm
        image: curlimages/curl:latest
        command:
        - /bin/sh
        - -c
        - |
          curl -X PUT "http://elasticsearch-master:9200/_ilm/policy/logs_policy" -H 'Content-Type: application/json' -d'
          {
            "policy": {
              "phases": {
                "hot": {
                  "min_age": "0ms",
                  "actions": {
                    "rollover": {
                      "max_age": "7d",
                      "max_size": "50gb"
                    },
                    "set_priority": {
                      "priority": 100
                    }
                  }
                },
                "warm": {
                  "min_age": "30d",
                  "actions": {
                    "shrink": {
                      "number_of_shards": 1
                    },
                    "forcemerge": {
                      "max_num_segments": 1
                    },
                    "set_priority": {
                      "priority": 50
                    }
                  }
                },
                "cold": {
                  "min_age": "60d",
                  "actions": {
                    "set_priority": {
                      "priority": 0
                    }
                  }
                },
                "delete": {
                  "min_age": "90d",
                  "actions": {
                    "delete": {}
                  }
                }
              }
            }
          }'
      restartPolicy: OnFailure
EOF

kubectl apply -f elasticsearch-ilm.yaml
```

### 7.3 アップデート手順

```bash
# コントローラーアップデート
docker build -t lambda-microservice/rust-controller:v1.0.1 .
docker tag lambda-microservice/rust-controller:v1.0.1 <REGISTRY_URL>/lambda-microservice/rust-controller:v1.0.1
docker push <REGISTRY_URL>/lambda-microservice/rust-controller:v1.0.1

kubectl set image deployment/rust-controller -n controller rust-controller=<REGISTRY_URL>/lambda-microservice/rust-controller:v1.0.1

# ランタイムアップデート
faas-cli build -f nodejs-runtime.yml --tag v1.0.1
faas-cli push -f nodejs-runtime.yml
faas-cli deploy -f nodejs-runtime.yml
```

## 8. トラブルシューティング

### 8.1 一般的な問題と解決策

| 問題 | 確認事項 | 解決策 |
|------|---------|--------|
| Podが起動しない | Pod状態とイベント | `kubectl describe pod <pod-name> -n <namespace>` |
| サービスに接続できない | サービスエンドポイント | `kubectl get svc -n <namespace>` |
| ログエラー | コンテナログ | `kubectl logs <pod-name> -n <namespace>` |
| パフォーマンス低下 | リソース使用率 | Grafanaダッシュボード確認 |
| データベース接続エラー | シークレット設定 | シークレット値の確認と更新 |

### 8.2 ログ収集

```bash
# コントローラーログ確認
kubectl logs -f deployment/rust-controller -n controller

# 特定のPodのログ確認
kubectl logs -f <pod-name> -n <namespace>

# 全Podのログをファイルに保存
kubectl get pods -n controller -o jsonpath='{.items[*].metadata.name}' | xargs -I{} sh -c "kubectl logs {} -n controller > {}.log"
```

### 8.3 デバッグモード有効化

```bash
# コントローラーのデバッグモード有効化
kubectl set env deployment/rust-controller -n controller LOG_LEVEL=debug

# 変更の適用
kubectl rollout restart deployment/rust-controller -n controller
```

## 9. CI/CD設定

### 9.1 GitHub Actions設定

```yaml
# .github/workflows/build-deploy.yml
name: Build and Deploy

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v2
    
    - name: Login to Container Registry
      uses: docker/login-action@v2
      with:
        registry: <REGISTRY_URL>
        username: ${{ secrets.REGISTRY_USERNAME }}
        password: ${{ secrets.REGISTRY_PASSWORD }}
    
    - name: Build and push Controller
      uses: docker/build-push-action@v4
      with:
        context: ./controller
        push: true
        tags: <REGISTRY_URL>/lambda-microservice/rust-controller:latest,<REGISTRY_URL>/lambda-microservice/rust-controller:${{ github.sha }}
    
    # 他のコンポーネントも同様に設定
```

### 9.2 ArgoCD設定

```yaml
# argocd-application.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: lambda-microservice
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/your-org/lambda-microservice.git
    targetRevision: HEAD
    path: kubernetes
  destination:
    server: https://kubernetes.default.svc
    namespace: controller
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
    - CreateNamespace=true
```

## 10. 本番環境への移行チェックリスト

- [ ] すべてのシークレットが安全に管理されているか
- [ ] バックアップ戦略が実装されているか
- [ ] 監視とアラートが設定されているか
- [ ] スケーリング設定が適切か
- [ ] セキュリティポリシーが適用されているか
- [ ] 障害復旧手順が文書化されているか
- [ ] パフォーマンステストが完了しているか
- [ ] ログ収集と分析が設定されているか
- [ ] CI/CDパイプラインが機能しているか
- [ ] ドキュメントが最新か
