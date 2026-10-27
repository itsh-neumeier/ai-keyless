#!/usr/bin/env bash
# Sichert die AI-Keyless-Datenbank konsistent (pg_dump, Custom-Format).
# Nutzung: scripts/backup.sh [ZIELVERZEICHNIS]
#   Umgebungsvariablen: AKL_COMPOSE_PROJECT (Standard: ai-keyless),
#   AKL_COMPOSE_FILES (Standard: deploy/compose/compose.yaml)
# Secrets (deploy/compose/secrets) werden NICHT gesichert – separat aufbewahren.
# Geräteidentitäten (Volume akl-devices, private Schlüssel) werden als
# akl-<zeit>.devices.tar mitgesichert → Sicherung wie ein Geheimnis behandeln.
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

# Geräteidentitäten: Volume nur lesend einhängen, Rechte und Eigentümer erhalten.
DEVICES_VOLUME="${PROJECT}_akl-devices"
DEVICES_SHA=null
if docker volume inspect "$DEVICES_VOLUME" >/dev/null 2>&1; then
  docker run --rm -v "$DEVICES_VOLUME:/devices:ro" alpine:3.20 \
    tar -C /devices --numeric-owner -cf - . >"$BASE.devices.tar"
  DEVICES_SHA="\"$(sha256sum "$BASE.devices.tar" | cut -d' ' -f1)\""
fi

cat >"$BASE.json" <<JSON
{"created_at":"$STAMP","schema_version":$SCHEMA,"sha256":"$SHA","format":"pg_dump-custom","devices_sha256":$DEVICES_SHA}
JSON
echo "$BASE.dump"
