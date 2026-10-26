# Session-Handoff

## Sitzung 2026-10-26 (Projektstart, AKL-M000 + AKL-M010)

**Erledigt**
- Referenzprojekt ausgewertet (nur öffentliche Doku + Commit-Metadaten ohne Blobs): Commit `4c71b73`, keine Lizenz → Clean Room.
- Monorepo angelegt: Rust-Workspace (`akl-core`, `akl-persistence`, `akl-api`), SvelteKit-3.1-SPA, Compose, Dockerfile, CI, Doku, ADR-001…003, Backlog.
- Docker-Daemon in der Cloud-Umgebung gestartet (Nutzer hat Docker für alle Entwicklertätigkeiten freigegeben).

**Tests (tatsächlich ausgeführt)**
- `cargo fmt --check`, `cargo clippy -D warnings`: sauber.
- `cargo test --workspace`: 16 Unit-/HTTP-Tests grün; 2 DB-Migrationstests grün gegen postgres:17-alpine (Docker).
- Web: prettier, eslint, svelte-check (0 Fehler/Warnungen), vitest 11 Tests, Build – grün.
- Playwright (Chromium) gegen laufende API: Dark/Light/Mobil, keine Konsolenfehler, kein horizontaler Überlauf, Live-Status korrekt.
- Docker-Image gebaut (`scripts/docker-build.sh`), `scripts/smoke-compose.sh` grün; DB-Ausfall → `readyz` 503 / `unavailable`, Wiederanlauf → 200.
- **Nicht ausgeführt:** GitHub-Actions-Lauf, gitleaks/cargo-audit/pnpm-audit (nur in CI definiert), arm64, Portainer.

**Stolpersteine**
- SvelteKit 3: `$lib` entfernt (→ `#lib/*.ts`), `svelte.config.js` nicht mehr unterstützt.
- Docker Hub 429 zeitweise; Build-Container brauchen in der Cloud Proxy + CA → `scripts/docker-build.sh`.
- Halb gestarteter Compose-Container nach Portkonflikt war nur am internen Netz → `--force-recreate`.

**Nächster Schritt**
Siehe PROJECT_STATE „Nächstes Entwicklungspaket“. Vorher Freigaben in OPEN_QUESTIONS klären (Q-001 Lizenz, Q-005 GitHub-Issues/PR).
