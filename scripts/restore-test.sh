#!/usr/bin/env bash
# Wiederherstellungsprobe: Daten anlegen → sichern → Volume löschen →
# wiederherstellen → Anmeldung mit alten Konten prüfen.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)
export AKL_SECRETS_DIR="$WORK/secrets" AKL_IMAGE=ai-keyless:dev AKL_ADMIN_PORT=${AKL_ADMIN_PORT:-18092}
export AKL_COMPOSE_PROJECT=akl-restore AKL_COMPOSE_FILES="$ROOT/deploy/compose/compose.yaml"
COMPOSE=(docker compose -p "$AKL_COMPOSE_PROJECT" -f "$AKL_COMPOSE_FILES")
BASE="http://127.0.0.1:${AKL_ADMIN_PORT}"
cleanup() { "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true; rm -rf "$WORK"; }
trap cleanup EXIT
fail() { echo "RESTORE-TEST FEHLER: $*" >&2; exit 1; }
ready() { for _ in $(seq 1 30); do curl -fsS "$BASE/readyz" >/dev/null 2>&1 && return 0; sleep 1; done; return 1; }

"$ROOT/scripts/init-secrets.sh" "$AKL_SECRETS_DIR" >/dev/null
"${COMPOSE[@]}" up -d --no-build --wait --wait-timeout 120
ready || fail "Stack nicht bereit"

# 1. Daten anlegen: Admin + weiterer Benutzer.
TOKEN=$("${COMPOSE[@]}" exec -T api ai-keyless-api setup-token | tail -1)
JAR="$WORK/jar"
CSRF=$(curl -fsS -c "$JAR" -H 'content-type: application/json' \
  -d "{\"token\":\"$TOKEN\",\"username\":\"admin\",\"password\":\"restore-test-pw-1\"}" \
  "$BASE/api/v1/auth/setup" | python3 -c "import json,sys; print(json.load(sys.stdin)['csrf_token'])")
curl -fsS -b "$JAR" -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
  -d '{"username":"zweiter","password":"restore-test-pw-2","role":"operator"}' \
  "$BASE/api/v1/users" >/dev/null || fail "Benutzer anlegen"

# 2. Sichern.
DUMP=$("$ROOT/scripts/backup.sh" "$WORK/backups")
[[ -s "$DUMP" ]] || fail "leere Sicherung"

# 3. Totalverlust simulieren: Volumes löschen, frisch starten.
"${COMPOSE[@]}" down -v >/dev/null
"${COMPOSE[@]}" up -d --no-build --wait --wait-timeout 120
ready || fail "frischer Stack nicht bereit"
curl -fsS "$BASE/api/v1/auth/state" | grep -q '"setup_required":true' || fail "Volume nicht leer"

# 4. Manipulierte Sicherung wird abgelehnt.
cp "$DUMP" "$WORK/tampered.dump"; cp "${DUMP%.dump}.json" "$WORK/tampered.json"
printf 'x' >>"$WORK/tampered.dump"
if "$ROOT/scripts/restore.sh" "$WORK/tampered.dump" >/dev/null 2>&1; then fail "manipulierte Sicherung akzeptiert"; fi

# 5. Wiederherstellen und prüfen.
"$ROOT/scripts/restore.sh" "$DUMP"
curl -fsS "$BASE/api/v1/auth/state" | grep -q '"setup_required":false' || fail "Einrichtung nach Restore offen"
for cred in admin:restore-test-pw-1 zweiter:restore-test-pw-2; do
  user=${cred%%:*}; pw=${cred#*:}
  curl -fsS -H 'content-type: application/json' -d "{\"username\":\"$user\",\"password\":\"$pw\"}" \
    "$BASE/api/v1/auth/login" | grep -q "\"username\":\"$user\"" || fail "Anmeldung $user nach Restore"
done
echo "Wiederherstellungsprobe erfolgreich"
