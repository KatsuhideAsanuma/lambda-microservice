#!/bin/bash

export DOCKER_HOST=unix:///var/run/docker.sock

"$@"
