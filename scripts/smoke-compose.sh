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
# Ohne Anmeldung kein Status; Einrichtung ist ausstehend.
[[ $(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/v1/system/status") == 401 ]] || fail "Status ohne Anmeldung"
curl -fsS "$BASE/api/v1/auth/state" | grep -q '"setup_required":true' || fail "auth/state"

# Einrichtung: Token per CLI (stdout, nicht Log), Admin anlegen, Sitzung nutzen.
TOKEN=$("${COMPOSE[@]}" exec -T api ai-keyless-api setup-token | tail -1)
[[ ${#TOKEN} -eq 43 ]] || fail "setup-token"
JAR="$WORK/cookies"
SETUP=$(curl -fsS -c "$JAR" -H 'content-type: application/json' \
  -d "{\"token\":\"$TOKEN\",\"username\":\"smoke\",\"password\":\"smoke-test-passwort\"}" \
  "$BASE/api/v1/auth/setup") || fail "Einrichtung"
echo "$SETUP" | grep -q '"role":"admin"' || fail "Admin-Rolle"
grep -q akl_session "$JAR" || fail "Sitzungscookie"
"${COMPOSE[@]}" exec -T api ai-keyless-api setup-token >/dev/null 2>&1 && fail "zweites Setup-Token nach Einrichtung"

STATUS=$(curl -fsS -b "$JAR" "$BASE/api/v1/system/status") || fail "Status mit Sitzung"
echo "$STATUS" | grep -q '"unofficial":true' || fail "unofficial-Kennzeichnung"
echo "$STATUS" | grep -q '"status":"ok"' || fail "Datenbankstatus"
# Ohne Lifecycle-Implementierung darf kein Gerätezustand behauptet werden.
echo "$STATUS" | grep -q '"state":null' || fail "erfundener Gerätezustand"
[[ $(curl -s -o /dev/null -w '%{http_code}' "$BASE/cameras") == 200 ]] || fail "SPA-Fallback"
[[ $(curl -s -o /dev/null -w '%{http_code}' "$BASE/api/v1/unknown") == 404 ]] || fail "API-404"
LOGS=$("${COMPOSE[@]}" logs api 2>&1)
grep -qF "$(cat "$AKL_SECRETS_DIR/db_password")" <<<"$LOGS" && fail "DB-Passwort im Log"
grep -qF "$TOKEN" <<<"$LOGS" && fail "Setup-Token im Log"

echo "Smoke-Test erfolgreich"
