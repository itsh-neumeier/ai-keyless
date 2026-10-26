# Session-Handoff

## Sitzung 2026-10-26 (Fortsetzung: Interview + Lizenz)

**Erledigt**
- Interview mit dem Projektverantwortlichen: Lizenz AGPL-3.0-or-later, Risiko Geräteimitation bewusst getragen,
  keine Testumgebung, Draft-PR erlaubt (Issues nicht), Plattform Linux-VM/x86_64-NAS, GPU NVIDIA < 12 GB / Intel/AMD / Coral,
  Provider Ollama → OpenAI-kompatibel → OpenAI Cloud → weitere Cloud. FAQ später.
- `LICENSE` (gnu.org), ADR-004, `docs/legal/RISK-ACCEPTANCE.md`, Lizenzmetadaten (Cargo, package.json, OCI-Label).
- API-Vertrag um `source_url` erweitert (AGPL §13), UI verlinkt den Quellcode unter System.
- Backlog: +6 Issues (Provider einzeln, GPU-Profil, Coral), ARM64 → P3.

**Tests:** cargo fmt/clippy sauber, 18 Rust-Tests (inkl. 2 DB) grün; Web lint/check/format, 12 vitest-Tests, Build grün;
Playwright-Sichtprüfung System-Seite (Link korrekt, keine Konsolenfehler).

**Hinweis:** Die Cloud-Umgebung wurde zwischendurch neu gestartet; Docker-Daemon musste erneut gestartet werden.


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
