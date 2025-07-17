#!/bin/bash


if docker compose version &> /dev/null; then
  export DOCKER_HOST=unix:///var/run/docker.sock
  
  CMD=$1
  shift
  
  docker compose $CMD "$@"
else
  docker-compose "$@"
fi
