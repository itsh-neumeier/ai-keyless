#!/usr/bin/env bash
# Startet den Compose-Stack mit Wegwerf-Secrets und prüft Kernzusagen.
# Erwartet ein lokal gebautes Image ai-keyless:dev (scripts/docker-build.sh).
set -euo pipefail
cd "$(dirname "$0")/.."

WORK=$(mktemp -d)
export AKL_SECRETS_DIR="$WORK/secrets" AKL_IMAGE=ai-keyless:dev AKL_ADMIN_PORT=${AKL_ADMIN_PORT:-18090}
COMPOSE=(docker compose -p akl-smoke -f deploy/compose/compose.yaml)
cleanup() { "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true; rm -rf "$WORK"; }
trap cleanup EXIT

scripts/init-secrets.sh "$AKL_SECRETS_DIR" >/dev/null
"${COMPOSE[@]}" up -d --no-build --wait --wait-timeout 120

BASE="http://127.0.0.1:${AKL_ADMIN_PORT}"
fail() { echo "SMOKE FEHLER: $*" >&2; "${COMPOSE[@]}" logs --tail 50 >&2 || true; exit 1; }

curl -fsS "$BASE/healthz" | grep -q '"ok"' || fail "healthz"
for _ in $(seq 1 30); do
  curl -fsS "$BASE/readyz" >/dev/null 2>&1 && break
  sleep 1
done
curl -fsS "$BASE/readyz" | grep -q '"ready":true' || fail "readyz"
STATUS=$(curl -fsS "$BASE/api/v1/system/status")
echo "$STATUS" | grep -q '"unofficial":true' || fail "unofficial-Kennzeichnung"
echo "$STATUS" | grep -q '"status":"ok"' || fail "Datenbankstatus"
# Ohne Lifecycle-Implementierung darf kein Gerätezustand behauptet werden.
echo "$STATUS" | grep -q '"state":null' || fail "erfundener Gerätezustand"
[[ $(curl -s -o /dev/null -w '%{http_code}' "$BASE/cameras") == 200 ]] || fail "SPA-Fallback"
[[ $(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/v1/unknown") == 404 ]] || fail "API-404"
"${COMPOSE[@]}" logs api 2>&1 | grep -q "$(cat "$AKL_SECRETS_DIR/db_password")" && fail "Secret im Log"

echo "Smoke-Test erfolgreich"
