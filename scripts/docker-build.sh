#!/usr/bin/env bash
# Baut das lokale Entwicklungsimage ai-keyless:dev.
# In Umgebungen mit HTTPS-Proxy (z. B. Claude-Code-Cloud) werden Proxy und CA
# automatisch durchgereicht; das CA landet nicht im Image (BuildKit-Secret).
set -euo pipefail
cd "$(dirname "$0")/.."
TAG=${1:-ai-keyless:dev}
args=(-f deploy/docker/Dockerfile -t "$TAG")
if [[ -n "${HTTPS_PROXY:-}" ]]; then
  args+=(--network host --build-arg "HTTPS_PROXY=$HTTPS_PROXY" --build-arg "https_proxy=$HTTPS_PROXY"
         --build-arg "NO_PROXY=${NO_PROXY:-}" --build-arg "no_proxy=${NO_PROXY:-}")
fi
CA=${AKL_EXTRA_CA:-/root/.ccr/ca-bundle.crt}
if [[ -r "$CA" ]]; then
  args+=(--secret "id=extra_ca,src=$CA")
fi
exec docker build "${args[@]}" .
