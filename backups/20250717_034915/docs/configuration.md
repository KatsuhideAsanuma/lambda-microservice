# 設定管理

このドキュメントでは、Lambda Microserviceの異なる環境での設定方法について説明します。

## ローカル開発

Dockerを使用しないローカル開発の場合、`.env.sample`ファイルを`.env`にコピーして必要に応じて変更します：

```bash
cp .env.sample .env
```

## Docker Compose

Docker Composeを使用する場合、機密情報はDocker Secretsを使用して管理されます：

- データベースURL: `./secrets/db_url.txt`
- Redis URL: `./secrets/redis_url.txt`
- Redis Cache URL: `./secrets/redis_cache_url.txt`

機密性の低い設定は、`docker-compose.yml`ファイルに環境変数として直接提供されます。

## Kubernetes

Kubernetes環境では、設定はConfigMapとSecretsを使用して管理されます：

- 機密性の低い設定: `kubernetes/controller/configmap.yaml`
- 機密情報: `kubernetes/controller/secrets.yaml`

環境固有の設定は以下に保存されます：

- 開発環境: `kubernetes/environments/dev/`
- 本番環境: `kubernetes/environments/prod/`

特定の環境の設定を適用するには：

```bash
kubectl apply -f kubernetes/environments/dev/
```
