#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

docker compose --env-file .env -f docker/docker-compose.yml down
docker compose --env-file .env -f docker/docker-compose.yml up -d --build
