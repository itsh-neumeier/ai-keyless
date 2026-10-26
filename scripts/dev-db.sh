#!/usr/bin/env bash
# Startet eine wegwerfbare PostgreSQL-Instanz für lokale Tests (nur Loopback).
# Nutzung:  scripts/dev-db.sh start|stop|url
set -euo pipefail

NAME=akl-dev-db
PORT=${AKL_DEV_DB_PORT:-55432}
# Nur lokales Wegwerf-Passwort für Tests – niemals produktiv verwenden.
URL="postgres://akl:akl-dev-only@127.0.0.1:${PORT}/akl"

case "${1:-start}" in
  start)
    if ! docker ps --format '{{.Names}}' | grep -qx "$NAME"; then
      docker rm -f "$NAME" >/dev/null 2>&1 || true
      docker run -d --name "$NAME" -p "127.0.0.1:${PORT}:5432" \
        -e POSTGRES_USER=akl -e POSTGRES_PASSWORD=akl-dev-only -e POSTGRES_DB=akl \
        postgres:17-alpine >/dev/null
    fi
    for _ in $(seq 1 30); do
      if docker exec "$NAME" pg_isready -U akl -d akl >/dev/null 2>&1; then
        echo "$URL"; exit 0
      fi
      sleep 1
    done
    echo "PostgreSQL nicht bereit" >&2; exit 1 ;;
  stop) docker rm -f "$NAME" >/dev/null ;;
  url) echo "$URL" ;;
  *) echo "Nutzung: $0 start|stop|url" >&2; exit 2 ;;
esac
