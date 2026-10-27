#!/usr/bin/env bash
# Ende-zu-Ende-Tests (Playwright) gegen einen frischen Compose-Stack.
# Erwartet ein gebautes Image ai-keyless:dev (scripts/docker-build.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

WORK=$(mktemp -d)
export AKL_SECRETS_DIR="$WORK/secrets" AKL_IMAGE=ai-keyless:dev AKL_ADMIN_PORT=${AKL_ADMIN_PORT:-18091}
COMPOSE=(docker compose -p akl-e2e -f "$ROOT/deploy/compose/compose.yaml" -f "$ROOT/deploy/compose/compose.e2e.yaml")
cleanup() {
  if [[ ${KEEP_STACK:-0} != 1 ]]; then "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true; rm -rf "$WORK"; fi
}
trap cleanup EXIT

"$ROOT/scripts/init-secrets.sh" "$AKL_SECRETS_DIR" >/dev/null
"${COMPOSE[@]}" up -d --no-build --wait --wait-timeout 120
for _ in $(seq 1 30); do
  curl -fsS "http://127.0.0.1:${AKL_ADMIN_PORT}/readyz" >/dev/null 2>&1 && break
  sleep 1
done

export E2E_BASE_URL="http://127.0.0.1:${AKL_ADMIN_PORT}"
E2E_SETUP_TOKEN=$("${COMPOSE[@]}" exec -T api ai-keyless-api setup-token | tail -1)
export E2E_SETUP_TOKEN E2E_PROVIDER_HOST=host.docker.internal

cd "$ROOT/tests/e2e"
[[ -d node_modules ]] || pnpm install --frozen-lockfile
pnpm exec playwright test "$@" || { "${COMPOSE[@]}" logs --tail 80 api >&2; exit 1; }
