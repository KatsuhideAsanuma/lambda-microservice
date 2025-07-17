#!/bin/bash


ENV=$1
if [ -z "$ENV" ]; then
  echo "使用法: $0 <environment>"
  echo "利用可能な環境: dev, test, prod"
  exit 1
fi

kubectl apply -f kubernetes/namespaces.yaml

echo "$ENV 環境の設定を適用しています..."
kubectl apply -f kubernetes/environments/$ENV/

kubectl apply -f kubernetes/runtimes/
kubectl apply -f kubernetes/functions/

echo "$ENV 環境へのデプロイが正常に完了しました！"
