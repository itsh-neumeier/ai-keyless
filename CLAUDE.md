# CLAUDE.md – AI-Keyless

Inoffizieller, unabhängiger, self-hosted AI-Prozessor für UniFi Protect (AI-Key-/AI-Port-
Kompatibilitätsmodus). **Nicht** von Ubiquiti. Projektsprache Deutsch; Code-Bezeichner Englisch.

## Sitzungsstart (Pflicht)

1. Lies: `AGENTS.md`, `docs/ai-memory/PROJECT_STATE.md`, `SESSION_HANDOFF.md`, `DECISIONS.md`,
   `OPEN_QUESTIONS.md`, `docs/adr/`, `docs/roadmap/BACKLOG.md` (bzw. GitHub-Issues, sobald angelegt).
2. `git status`, Branches, offene PRs prüfen. Fremde uncommittete Änderungen nie überschreiben.
3. Höchste freigegebene, nicht blockierte Aufgabe wählen; vor Änderungen Ziel, Abhängigkeiten,
   Akzeptanzkriterien nennen.
4. Am Ende: `SESSION_HANDOFF.md` + `PROJECT_STATE.md` aktualisieren; Bericht (erledigt, Dateien,
   Tests mit Ergebnis, Risiken, nötige Freigaben, nächstes Issue).

## Unveränderliche Regeln

- **Wahrheit:** Keine erfundenen Testergebnisse, Fähigkeiten, Versionskompatibilitäten oder
  UI-Zustände. Unbekannt bleibt `null`/„unbekannt“. Jede Fähigkeitsaussage trägt eine
  Evidenzstufe (`planned`, `implemented`, `synthetic_verified`, `native_verified`,
  `unsupported`, `needs_evidence`) und eine Version. HTTP 200 ≠ native Persistenz.
- **Clean Room (ADR-002):** Kein Code aus `Olli0103/unifi-ai-key-emulator` (keine Lizenz) oder
  anderen unlizenzierten Quellen; keine Firmware, Schlüssel, Modelle, Hersteller-Assets.
  Neue Quellen vor Nutzung in `docs/legal/PROVENANCE.md` eintragen.
- **Profile strikt getrennt (ADR-003):** `ai_key` ≠ `ai_port`.
- **Autonomiegrenze – Freigabe nötig vor:** kostenpflichtigen Modellaufrufen, echten
  Controller-Änderungen/Live-Tests, geheimnisrelevanten Aktionen, Upload privater Medien,
  Öffnen öffentlicher PRs, GitHub-Issues/-Veröffentlichung, Deployments, Releases.
- **Secrets** nie in Git, Logs, API-Antworten oder Browserstorage. Nur `*_FILE`/Docker-Secrets.
- Keine Fake-Daten im Normalbetrieb; Demo nur im getrennten Demo-Modus.

## Befehle

```bash
# Docker-Daemon in Cloud-Sessions ggf. starten (Docker ist für alle Entwicklertätigkeiten freigegeben):
docker info >/dev/null 2>&1 || (sudo dockerd >/tmp/dockerd.log 2>&1 &)

# Rust
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
# DB-Tests gegen echte PostgreSQL (Docker):
export AKL_TEST_DATABASE_URL=$(scripts/dev-db.sh start) AKL_REQUIRE_DB_TESTS=1 && cargo test -p akl-persistence

# Web (apps/web)
pnpm install && pnpm run format:check && pnpm lint && pnpm check && pnpm test && pnpm build

# Lokal starten (API + gebaute UI, http://127.0.0.1:8090)
AKL_DATABASE_URL=$(scripts/dev-db.sh url) AKL_WEB_DIR=apps/web/build AKL_BIND=127.0.0.1:8090 cargo run -p akl-api
# UI-Entwicklung mit Hot-Reload (Proxy auf :8090): cd apps/web && pnpm dev

# Container
scripts/docker-build.sh            # Image ai-keyless:dev (reicht Proxy/CA in Cloud-Umgebung durch)
scripts/smoke-compose.sh           # Compose-Stack mit Wegwerf-Secrets prüfen

# Backlog
python3 scripts/gen-backlog-md.py  # BACKLOG.md aus backlog.json
```

## Stack-Hinweise (verifiziert 2026-10-26)

- SvelteKit **3.1**: Konfiguration im Vite-Plugin (`vite.config.ts`), **kein** `svelte.config.js`;
  `$lib` entfällt → `#lib/...` mit expliziter Endung (`#lib/nav.ts`, `#lib/i18n/index.svelte.ts`);
  Links über `resolve()` aus `$app/paths`; `tsconfig.json` erweitert `$app/tsconfig`.
- TypeScript bleibt 6.0.x (Peer von Kit 3.1).
- sqlx **0.9**: dynamisches SQL nur mit `sqlx::AssertSqlSafe` und begründetem Kommentar.
- Docker Hub kann 429 liefern → kurz warten und erneut versuchen.

## Struktur

`apps/api` (Axum) · `apps/web` (SvelteKit-SPA) · `crates/core` (Domäne, kein I/O) ·
`crates/persistence` (SQLx, Migrationen) · `deploy/` · `docs/` · `scripts/` · `tests/`.
