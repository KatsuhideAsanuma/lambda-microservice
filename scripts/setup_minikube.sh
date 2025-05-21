#!/bin/bash
set -e

echo "Setting up Minikube for local development..."

if ! command -v minikube &> /dev/null; then
    echo "Minikubeがインストールされていません。インストール方法については以下を参照してください："
    echo "https://minikube.sigs.k8s.io/docs/start/"
    exit 1
fi

minikube start --memory=4096 --cpus=2

kubectl create namespace lambda-microservice || echo "名前空間lambda-microserviceは既に存在します"

echo "ランタイムサービスをデプロイしています..."

export REGISTRY=localhost:5000
export TAG=latest

for runtime in nodejs python rust; do
    cat kubernetes/runtimes/${runtime}-deployment.yaml | \
    sed "s/\${REGISTRY}/${REGISTRY}/g" | \
    sed "s/\${TAG}/${TAG}/g" | \
    kubectl apply -f - -n lambda-microservice
done

echo "Minikubeセットアップが完了しました！"
