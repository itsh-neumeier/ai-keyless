# Session-Handoff

## Sitzung 2026-10-27 (Lab-PC, TP-000)

- Claude-Code-Sitzung auf dem Windows-PC (Remote Control); Lab-Stack (API, DB, Ollama) lief bereits, `/readyz` ready.
- TP-000 durchgeführt, nur lesend, ohne Login: `docs/compatibility/evidence/2026-10-27-tp000-umgebung.md`.
  Protect 7.2.105 / UniFi OS 5.1.33 / 8 × G5 Turret Ultra FW 5.5.31 (vom Projektverantwortlichen aus der UI abgelesen).
- T600-Messung `granite3.2-vision-4k`: 20/41 Schichten GPU, Kaltstart 131,7 s (> API-Limit 120 s), warm 0,49–0,66 s → `PROVIDERS.md`.
- Coral: `usbipd` hängt `18d1:9302` an WSL, im privilegierten Container sichtbar; kein Modelltest.
- Diese Sitzung ist ab jetzt Hauptsitzung (Cloud-Sitzung beendet). `gh` 2.103.3 per winget installiert, angemeldet.
- **Entschieden (DECISIONS):** Protect-Update auf 7.3.70 vor TP-001 (Projektverantwortlicher führt durch, danach Versionen neu ablesen);
  TP-001 im Produktivnetz mit Auflagen; Kaltstart: keep_alive + Warm-up im Lab **und** konfigurierbares Provider-Zeitlimit.
- Nächste Schritte: Kaltstart-Umsetzung, dann #2 (Netzwerkmodi, Plan im Issue-Kommentar), dann #3/#4.

**Fortsetzung (gleiche Sitzung) – offene PRs, alle Draft, Merge nach Freigabe:**

| PR | Inhalt | Tests |
|---|---|---|
| #39 | TP-000-Nachweis, Entscheidungen, AI-Memory | Doku |
| #40 | AKL-M080-06: `OLLAMA_KEEP_ALIVE=-1`, `lab.ps1 warm`, Lab-Zeitlimit 300 s | Lab-PC: warm → „Red“, UNTIL Forever |
| #41 | AKL-M080-01: `akl-netprobe` im Image, `scripts/netlab.sh`, `lab.ps1 netlab`, CI-Schritt, `NETWORKING.md` | Lab-PC 3× + CI grün |
| #42 | AKL-M040-01 (#3): `crates/device-lifecycle` Identität, `AKL_DEVICE_DIR`, Volume `akl-devices` | 9 Unit-Tests, Workspace grün |
| #43 | AKL-M040-02 (#4): Lifecycle-Service, Transition-Log, Kill-Switch, `/api/v1/devices` – **Basis #42** | DB-Tests gegen PostgreSQL 17 grün |

- Netzwerk-Ergebnis: host/macvlan ✓; bridge ungeeignet (Docker Desktop kein Broadcast; Linux nur über `docker-proxy`
  mit Gateway 172.18.0.1 als Absender). Echtes LAN unter Docker Desktop: `needs_evidence` (Issue #2 Schritt 3, eigene Freigabe).
- Entwicklung auf dem PC: kein lokales Rust; `cargo` läuft in `rust:1.97-slim-bookworm` mit Volumes `akl-cargo-target`/`akl-cargo-registry`,
  DB-Tests gegen Wegwerf-`postgres:17-alpine` im Docker-Netz `akl-test` (`AKL_TEST_DATABASE_URL=postgres://akl:test@akl-testdb:5432/akl`).
  Python 3.13 per winget installiert.
- Auto-Mode blockiert `git push --force*` und `gh issue create` → Folge-Issue „Backup der Geräteidentitäten“ steht nur im PR-Text von #42.
- Lab-Stack: API zuletzt mit `main`-Overlay neu erstellt → Zeitlimit wieder 120 s, bis #40 gemergt ist; Ollama hält das Modell weiter (keep_alive).
- Nächstes: Reviews/Merges #39–#43; dann AKL-M030-03 (Fixtures) bzw. AKL-M040-03 (Discovery gegen Simulator, braucht #41-Ergebnis);
  TP-001 erst nach Protect-Update 7.3.70 und eigener Freigabe.

## Sitzung 2026-10-27 (Windows-Lab)

- PR #37 gemergt (Windows-Lab-Doku, `.gitattributes`, TLS-Pinning, `akl-protect-sim`, TP-000 freigegeben).
- Neu: `scripts/lab.ps1` (up/token/model/status/logs/down) – reines PowerShell, ASCII-only für PS 5.1.
  **Nicht ausgeführt** (kein PowerShell in der Cloud-Umgebung); Erstlauf durch den Projektverantwortlichen auf dem PC steht aus.
- `LAB-SETUP.md` auf PowerShell umgestellt (Anlass: `\`-Fortsetzung und `.sh`-Skripte schlugen in PowerShell fehl).
- Nächste Cloud-Aufgaben: #2 Netzwerk-Lab, #9/#10 Simulator/Fixtures, #3/#4 Geräteidentität/-lebenszyklus.

## Sitzung 2026-10-26/27 (Abarbeitung Backlog)

**Erledigt (alle auf `claude/ai-keyless-unifi-protect-w0ntn0`, PR #1)**
- `8a8839c` AKL-M020-08 Playwright-E2E gegen Compose-Stack in der CI (fand 2 echte Fehler: leere Allowlist aus Compose-Vorgabe, mobiler Überlauf durch sr-only).
- `41f0d50` AKL-M080-04 Backup/Restore + Wiederherstellungsprobe (CI).
- `c024827` AKL-M030-04 Testplan-Vorlage, Nachweisformat, TP-001-Entwurf.
- `5057bca` AKL-M020-04 Konten deaktivieren, Passwort ändern.
- `7a7eac1` AKL-M020-01/02 Authentifizierung (Setup-Token per CLI, Argon2id, Sitzungen, CSRF, Origin-Guard, Rollen, Drosselung, Audit, UI).
- `38af92a` AKL-M030-01 Versionsprofile (`crates/protect-compat`), API + Geräte-Seite.
- `06d5e69` AKL-M060-02/05/06 Provider mit SSRF-Schutz (`crates/providers`), UI KI-Modelle, echte Messung mit Ollama.
- `8bf7563` AKL-M060-01/03 Persistente Queue, Worker, Stundenlimit, UI Verarbeitung.

**Tests (ausgeführt, Stand `5057bca`):** 75 Rust-Tests, 18 Web-Tests, 5 E2E-Tests, Smoke- und Wiederherstellungsprobe lokal grün; CI bis `c024827` grün (inkl. E2E/Restore).
Frühere Zählung: 73 Rust-Tests (inkl. DB-Tests gegen PostgreSQL 17 in Docker), 18 Web-Tests, Lint/Typecheck,
Compose-Smoke-Test mit Einrichtung im Container, Playwright-Abläufe (Setup/Login/Benutzer, Geräte, KI-Modelle gegen echtes Ollama,
Queue → Worker → Ollama), gitleaks. CI auf GitHub für alle Commits bis `06d5e69` grün.

**Umgebungshinweise:** Das Platzkontingent ist knapp (≈13 GB): `docker builder prune -af` und `rm -rf target/debug/incremental` geben Platz frei; das Ollama-Image (5,4 GB) liegt noch lokal.
 Die Cloud-Umgebung startet gelegentlich neu → Docker-Daemon fehlt; `scripts/dev-db.sh start` startet ihn jetzt selbst.
Ollama-Testcontainer: `docker run -d --name akl-ollama --network host -e OLLAMA_HOST=0.0.0.0:11434 …` (siehe PROVIDERS.md); erreichbar für die API über `http://172.17.0.1:11434`.
Lokaler Test-Admin in der Dev-DB: `admin` / `korrekt-pferd-batterie` (nur Wegwerf-Dev-DB).

**Offene Freigaben:** Kostenrahmen Cloud-Provider (Q-011), weitere Cloud-Anbieter (Q-013), GPU-Modelle (Q-014), GitHub-Issues (Q-005), Testumgebung (Q-003).


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
