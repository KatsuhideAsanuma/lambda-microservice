#!/bin/bash
set -e

echo "Logging in to OpenFaaS..."
PASSWORD=$(kubectl get secret -n openfaas basic-auth -o jsonpath="{.data.basic-auth-password}" | base64 --decode)
echo -n $PASSWORD | faas-cli login --username admin --password-stdin

echo "Building and deploying runtime functions..."
faas-cli build -f ./openfaas/functions.yml
faas-cli push -f ./openfaas/functions.yml
faas-cli deploy -f ./openfaas/functions.yml

echo "Runtime functions deployed!"
