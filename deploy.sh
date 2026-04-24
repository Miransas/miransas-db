#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo ">>> Building & starting containers"
docker compose --env-file .env -f docker/docker-compose.yml up -d --build

echo ">>> Waiting 5s"
sleep 5

echo ">>> Reconnecting to binboi network (for Caddy routing)"
docker network connect binboi_binboi-net miransas-db-backend 2>/dev/null || echo "(already connected)"

echo ">>> Status"
docker ps --filter name=miransas-db --format "table {{.Names}}\t{{.Status}}"
