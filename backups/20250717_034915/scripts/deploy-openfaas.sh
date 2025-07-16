#!/bin/bash
set -e

if ! [ -x "$(command -v faas-cli)" ]; then
  echo "Installing OpenFaaS CLI..."
  curl -sLS https://cli.openfaas.com | sudo sh
fi

if [ -d "./kubernetes/openfaas" ]; then
  echo "Deploying OpenFaaS to Kubernetes..."
  kubectl apply -f ./kubernetes/openfaas/namespaces.yaml
  kubectl apply -f ./kubernetes/openfaas/gateway.yaml
  kubectl apply -f ./kubernetes/openfaas/function-controller.yaml
  
  kubectl rollout status -n openfaas deploy/gateway
  kubectl rollout status -n openfaas deploy/faas-netes
  
  kubectl apply -f ./kubernetes/functions/
fi

echo "OpenFaaS deployment completed!"
