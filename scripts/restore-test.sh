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
# Geräteprofil mit Identität (privater Schlüssel im Volume akl-devices).
FINGERPRINT=$(curl -fsS -b "$JAR" -H 'content-type: application/json' -H "x-csrf-token: $CSRF" \
  -d '{"kind":"ai_key","display_name":"Restore-Probe"}' "$BASE/api/v1/devices" |
  python3 -c "import json,sys; print(json.load(sys.stdin)['fingerprint'])") || fail "Geräteprofil anlegen"
[[ ${#FINGERPRINT} -eq 95 ]] || fail "Fingerprint unerwartet: $FINGERPRINT"

# 2. Sichern.
DUMP=$("$ROOT/scripts/backup.sh" "$WORK/backups")
[[ -s "$DUMP" ]] || fail "leere Sicherung"
[[ -s "${DUMP%.dump}.devices.tar" ]] || fail "Identitäten nicht gesichert"

# 3. Totalverlust simulieren: Volumes löschen, frisch starten.
"${COMPOSE[@]}" down -v >/dev/null
"${COMPOSE[@]}" up -d --no-build --wait --wait-timeout 120
ready || fail "frischer Stack nicht bereit"
curl -fsS "$BASE/api/v1/auth/state" | grep -q '"setup_required":true' || fail "Volume nicht leer"

# 4. Manipulierte Sicherung wird abgelehnt.
cp "$DUMP" "$WORK/tampered.dump"; cp "${DUMP%.dump}.json" "$WORK/tampered.json"
printf 'x' >>"$WORK/tampered.dump"
if "$ROOT/scripts/restore.sh" "$WORK/tampered.dump" >/dev/null 2>&1; then fail "manipulierte Sicherung akzeptiert"; fi
# Manipulierte Identitätssicherung (Dump unverändert) wird ebenfalls abgelehnt.
cp "$DUMP" "$WORK/tampered2.dump"; cp "${DUMP%.dump}.json" "$WORK/tampered2.json"
cp "${DUMP%.dump}.devices.tar" "$WORK/tampered2.devices.tar"; printf 'x' >>"$WORK/tampered2.devices.tar"
if "$ROOT/scripts/restore.sh" "$WORK/tampered2.dump" >/dev/null 2>&1; then fail "manipulierte Identitätssicherung akzeptiert"; fi

# 5. Wiederherstellen und prüfen.
"$ROOT/scripts/restore.sh" "$DUMP"
curl -fsS "$BASE/api/v1/auth/state" | grep -q '"setup_required":false' || fail "Einrichtung nach Restore offen"
for cred in admin:restore-test-pw-1 zweiter:restore-test-pw-2; do
  user=${cred%%:*}; pw=${cred#*:}
  curl -fsS -H 'content-type: application/json' -d "{\"username\":\"$user\",\"password\":\"$pw\"}" \
    "$BASE/api/v1/auth/login" | grep -q "\"username\":\"$user\"" || fail "Anmeldung $user nach Restore"
done
# Identität unverändert (gleicher Fingerprint, Schlüssel passt laut API zum Zertifikat).
curl -fsS -c "$JAR" -H 'content-type: application/json' \
  -d '{"username":"admin","password":"restore-test-pw-1"}' "$BASE/api/v1/auth/login" >/dev/null
AFTER=$(curl -fsS -b "$JAR" "$BASE/api/v1/devices" |
  python3 -c "import json,sys; print(json.load(sys.stdin)['profiles'][0]['fingerprint'])") || fail "Geräteliste nach Restore"
[[ "$AFTER" == "$FINGERPRINT" ]] || fail "Fingerprint nach Restore: $AFTER ≠ $FINGERPRINT"
# Rechte und Eigentümer (nonroot = 65532) erhalten.
MODES=$(docker run --rm -v "${AKL_COMPOSE_PROJECT}_akl-devices:/d:ro" alpine:3.20 sh -c \
  'stat -c "%a %u" /d; stat -c "%a %u" /d/*/key.pem')
[[ "$MODES" == $'700 65532\n600 65532' ]] || fail "Rechte nach Restore: $MODES"
echo "Wiederherstellungsprobe erfolgreich"
