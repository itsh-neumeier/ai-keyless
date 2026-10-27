#!/usr/bin/env bash
# Sichert die AI-Keyless-Datenbank konsistent (pg_dump, Custom-Format).
# Nutzung: scripts/backup.sh [ZIELVERZEICHNIS]
#   Umgebungsvariablen: AKL_COMPOSE_PROJECT (Standard: ai-keyless),
#   AKL_COMPOSE_FILES (Standard: deploy/compose/compose.yaml)
# Secrets (deploy/compose/secrets) werden NICHT gesichert – separat aufbewahren.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST=${1:-$ROOT/backups}
PROJECT=${AKL_COMPOSE_PROJECT:-ai-keyless}
read -r -a FILES <<<"${AKL_COMPOSE_FILES:-$ROOT/deploy/compose/compose.yaml}"
COMPOSE=(docker compose -p "$PROJECT")
for f in "${FILES[@]}"; do COMPOSE+=(-f "$f"); done

mkdir -p "$DEST"
chmod 700 "$DEST"
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
BASE="$DEST/akl-$STAMP"
umask 077

"${COMPOSE[@]}" exec -T db pg_dump -U akl -d akl --format=custom --no-owner >"$BASE.dump"
SCHEMA=$("${COMPOSE[@]}" exec -T db psql -U akl -d akl -Atc \
  "SELECT max(version) FROM _sqlx_migrations WHERE success")
SHA=$(sha256sum "$BASE.dump" | cut -d' ' -f1)
cat >"$BASE.json" <<JSON
{"created_at":"$STAMP","schema_version":$SCHEMA,"sha256":"$SHA","format":"pg_dump-custom"}
JSON
echo "$BASE.dump"
