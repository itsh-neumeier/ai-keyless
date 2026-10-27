# syntax=docker/dockerfile:1.7
# AI-Keyless – Management-API inkl. statisch gebauter Web-UI (ADR-001).
# Build-Kontext: Repository-Wurzel.

ARG NODE_IMAGE=node:22.22-alpine
ARG RUST_IMAGE=rust:1.97-slim-bookworm
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian12:nonroot

FROM ${NODE_IMAGE} AS web
WORKDIR /src/apps/web
RUN --mount=type=secret,id=extra_ca,required=false \
    if [ -s /run/secrets/extra_ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/extra_ca; fi; \
    corepack enable && corepack prepare pnpm@10.28.0 --activate
COPY apps/web/package.json apps/web/pnpm-lock.yaml apps/web/.npmrc ./
# Optionales Firmen-/Proxy-CA als BuildKit-Secret (landet nicht im Image).
RUN --mount=type=secret,id=extra_ca,required=false \
    if [ -s /run/secrets/extra_ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/extra_ca; fi; \
    pnpm install --frozen-lockfile
COPY apps/web/ ./
RUN pnpm run build

FROM ${RUST_IMAGE} AS api
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps/api apps/api
COPY crates crates
RUN --mount=type=secret,id=extra_ca,required=false \
    if [ -s /run/secrets/extra_ca ]; then export CARGO_HTTP_CAINFO=/run/secrets/extra_ca; fi; \
    cargo build --release --locked -p akl-api \
 && cp target/release/ai-keyless-api /ai-keyless-api \
 && mkdir -m 0700 /devices

FROM ${RUNTIME_IMAGE}
LABEL org.opencontainers.image.title="AI-Keyless" \
      org.opencontainers.image.description="Inoffizieller, unabhängiger AI-Prozessor für UniFi Protect (nicht von Ubiquiti)" \
      org.opencontainers.image.source="https://github.com/itsh-neumeier/ai-keyless" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later"
COPY --from=api /ai-keyless-api /usr/local/bin/ai-keyless-api
COPY --from=web /src/apps/web/build /srv/web
# Geräteidentitäten (privater Schlüssel je Profil, AKL-M040-01). Gehört nonroot,
# damit ein leeres benanntes Volume diese Rechte übernimmt.
COPY --from=api --chown=nonroot:nonroot /devices /var/lib/ai-keyless/devices
ENV AKL_BIND=0.0.0.0:8090 \
    AKL_WEB_DIR=/srv/web \
    AKL_DEVICE_DIR=/var/lib/ai-keyless/devices \
    AKL_LOG_FORMAT=json
USER nonroot:nonroot
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/ai-keyless-api", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/ai-keyless-api"]
