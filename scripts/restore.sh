#!/usr/bin/env bash
# Stellt eine Sicherung aus scripts/backup.sh wieder her.
# Nutzung: scripts/restore.sh PFAD/akl-<zeit>.dump
# Ablauf: Prüfsumme prüfen → API stoppen → Datenbank ersetzen → API starten →
# Readiness und Schema-Version prüfen. Nur auf gleiche oder neuere AI-Keyless-
# Versionen wiederherstellen (Migrationen laufen beim Start vorwärts).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DUMP=${1:?Pfad zur .dump-Datei angeben}
META=${DUMP%.dump}.json
PROJECT=${AKL_COMPOSE_PROJECT:-ai-keyless}
PORT=${AKL_ADMIN_PORT:-8090}
read -r -a FILES <<<"${AKL_COMPOSE_FILES:-$ROOT/deploy/compose/compose.yaml}"
COMPOSE=(docker compose -p "$PROJECT")
for f in "${FILES[@]}"; do COMPOSE+=(-f "$f"); done

[[ -r "$DUMP" && -r "$META" ]] || { echo "Dump oder Metadaten fehlen" >&2; exit 2; }
EXPECTED=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['sha256'])" "$META")
SCHEMA=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['schema_version'])" "$META")
ACTUAL=$(sha256sum "$DUMP" | cut -d' ' -f1)
[[ "$EXPECTED" == "$ACTUAL" ]] || { echo "Prüfsumme stimmt nicht – Abbruch" >&2; exit 3; }

# Geräteidentitäten (ab Sicherungen mit devices_sha256; ältere haben keine).
DEVICES_TAR=${DUMP%.dump}.devices.tar
DEVICES_EXPECTED=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('devices_sha256') or '')" "$META")
if [[ -n "$DEVICES_EXPECTED" ]]; then
  [[ -r "$DEVICES_TAR" ]] || { echo "Identitätssicherung fehlt: $DEVICES_TAR" >&2; exit 2; }
  [[ "$(sha256sum "$DEVICES_TAR" | cut -d' ' -f1)" == "$DEVICES_EXPECTED" ]] ||
    { echo "Prüfsumme der Identitätssicherung stimmt nicht – Abbruch" >&2; exit 3; }
fi

"${COMPOSE[@]}" stop api >/dev/null
"${COMPOSE[@]}" up -d --wait db >/dev/null
"${COMPOSE[@]}" exec -T db psql -U akl -d postgres -qc \
  "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'akl' AND pid <> pg_backend_pid();" >/dev/null
"${COMPOSE[@]}" exec -T db dropdb -U akl --if-exists akl
"${COMPOSE[@]}" exec -T db createdb -U akl akl
"${COMPOSE[@]}" exec -T db pg_restore -U akl -d akl --no-owner --exit-on-error <"$DUMP"
if [[ -n "$DEVICES_EXPECTED" ]]; then
  # Volume leeren und mit Originalrechten/-eigentümern füllen (API ist gestoppt).
  docker run --rm -i -v "${PROJECT}_akl-devices:/devices" alpine:3.20 sh -c \
    'find /devices -mindepth 1 -delete && tar -C /devices --numeric-owner -xpf -' <"$DEVICES_TAR"
fi
"${COMPOSE[@]}" up -d --wait api >/dev/null

for _ in $(seq 1 30); do
  curl -fsS "http://127.0.0.1:${PORT}/readyz" >/dev/null 2>&1 && break
  sleep 1
done
NOW=$(curl -fsS "http://127.0.0.1:${PORT}/readyz" | python3 -c "import json,sys; print(json.load(sys.stdin)['database']['schema_version'])")
if (( NOW < SCHEMA )); then
  echo "Schema-Version nach Restore ($NOW) älter als Sicherung ($SCHEMA)" >&2; exit 4
fi
echo "Wiederhergestellt: Schema $SCHEMA → $NOW"
