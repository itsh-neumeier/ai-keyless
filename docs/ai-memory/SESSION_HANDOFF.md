# Session-Handoff

## Sitzung 2026-10-27, Nacht (Lab-PC) – PRs #51/#52, API-Anbindung AKL-M040-07

- #51 (M040-03) und #52 (M040-04, gestapelt) als Draft geöffnet, CI auf beiden vollständig grün.
  **Merge vom Nutzer freigegeben, aber von der Rechteprüfung („Merge ohne Review“) blockiert** – Merge macht der
  Projektverantwortliche selbst (oder erlaubt die Aktion). Danach Basis von #52 auf `main` umstellen.
- Neuer Branch `feat/AKL-M040-07-api-wiring` (auf #52): `crates/device-lifecycle/src/runtime.rs` (Supervisor für das
  aktive `ai_key`-Profil, startet/stoppt Discovery, Adoptions-Endpunkt, Kontrollkanal), Konfiguration in
  `apps/api/src/config.rs` (`AKL_DEVICE_EMULATION`, `AKL_DISCOVERY_BIND`, `AKL_ADOPTION_BIND`,
  `AKL_CONTROLLER_{IP,PORT,FINGERPRINT}`; leere Werte = nicht gesetzt), Start in `main.rs`, Compose-Variablen,
  Betriebsdoku. Standard **aus**; ohne bestätigten Controller nur Discovery.
- Tests: Workspace fmt/clippy grün; `akl-api` + `akl-device-lifecycle` alle grün mit PG 17, neu `tests/runtime.rs`
  (Ende-zu-Ende Probe → Adoption → online → Kill-Switch schließt Endpunkte) und Konfigurationstest.
- Offen: Identitäts-Backup enthält das Controller-Token (in Betriebsdoku vermerkt, Entscheidung offen),
  API/UI „Bindung entfernen“, Credential-Rotation, Lab-Compose für host/macvlan, danach TP-001 mit Freigabe.

## Sitzung 2026-10-27, spät (Lab-PC) – AKL-M040-03/04 synthetisch umgesetzt

- Grundlage: `docs/roadmap/HANDOFF-AKL-M040-03-04.md`. Zwei gestapelte Branches, **nicht gepusht, kein PR**
  (Freigabe nötig): `feat/AKL-M040-03-discovery-responder` → `feat/AKL-M040-04-adoption`.
- **M040-03** `crates/device-lifecycle/src/discovery.rs`: Responder antwortet nur auf v1/v2-Probe, Unicast nur an lokale
  Absender, 1 Antwort/s je Absender; erste Antwort `unconfigured → discovered`; Kill-Switch/`disabled` ⇒ stumm.
  Feldwerte neutral (`AKL-SYNTH`), MAC/Konfigurationsstatus nur wenn gesetzt (Kodierung offen, DISC-H07).
- **M040-04** `adoption.rs` + `control.rs`: TLS-Endpunkt mit eigenem Zertifikat (Port 8080 nur Konstante), nur vom
  vertrauten Controller (IP + Pin); gepinnte Rückverbindung **vor** Token-Speicherung; Token als `controller.token`
  (0600) im Identitätsverzeichnis. Kontrollkanal mit Keepalive, `reconnecting/offline`, Neustart-Readback,
  Pin-Wechsel ⇒ `error`. Zeilenformat (JSON-Zeilen) ist **synthetischer Platzhalter**, nicht UCP4.
- `akl-protect-sim`: `SimCert` (Neustart mit gleichem Fingerprint), Verbindungen enden mit dem Sim, `send_line_pinned`,
  `discovery_probe`.
- Tests: Workspace fmt/clippy `-D warnings` grün, alle Tests grün mit PG 17 (`AKL_REQUIRE_DB_TESTS=1`); neu 5+4
  Discovery- und 7 Adoptionstests, 5 Wiederholungen ohne Ausreißer. Gebaut im Container `rust:1-bookworm`
  (keine Rust-Toolchain auf dem Windows-Host).
- **Offen:** Verdrahtung in `akl-api` (opt-in, Konfiguration Controller-Pin), Credential-Rotation (M040-04-Titel),
  API/UI zum Entfernen der Bindung (`forget_binding` existiert), Identitäts-Backup enthält jetzt auch das Token
  (prüfen, ob gewollt). Danach TP-001 mit eigener Freigabe.

## Sitzung 2026-10-27, Abend (Lab-PC) – Merges + Übergabe

- **#39–#48 alle nach `main` gemergt** (Merge-Commits, keine Squashes); `main`-CI danach grün
  (Rust, Web, Abhängigkeiten/Secrets, Container inkl. E2E + Wiederherstellungsprobe).
  Zusätzlich gemergt ggü. der Vormittagskette: #47 (AKL-M020-06 a11y/axe), #48 (AKL-M020-07 CSP ohne `unsafe-inline`).
- Lab-Stack mit `scripts/lab.ps1 up` auf den neuen Image-Stand gebracht: `/readyz` Schema `20261027000004`,
  CSP-Header mit Hashes, `theme-init.js` 200, Volume `akl-devices` eingehängt, Kill-Switch aus, Modell geladen.
- **Geräte-Emulation gegenüber Protect (Discovery-Antwort #5/AKL-M040-03, Adoption #6/AKL-M040-04) ist weiterhin
  NICHT implementiert** und wurde in dieser Sitzung nicht gebaut (Sicherheitsfilter stoppte die Arbeit wiederholt).
  Übergabebeschreibung dafür: `docs/roadmap/HANDOFF-AKL-M040-03-04.md` (PR #49). Eine Folgesitzung/ein Mensch setzt es um.
- Nutzerwunsch offen: „AI-Key-Emulation an der echten UNVR testen“ → geht erst nach M040-03/04 + TP-001-Freigabe.

## Sitzung 2026-10-27, Nachmittag (Lab-PC)

**PR-Kette – alle Draft, CI grün, nicht gemergt.** Merge-Reihenfolge (lokal geprüft: konfliktfrei auf `main`):
#39 → #40 → #41 → #42 → #43 → #44 → #45 → #46. Konflikte wurden per Merge in die PR-Branches gelöst (kein Force-Push):
`lab.ps1` (#40/#41), `DECISIONS.md` + Evidence-README (#39/#44), `Dockerfile` (#41/#42), danach #42 → #43 → #45 → #46 durchgereicht.
Kombinierter Endstand: Rust fmt/clippy 0 Befunde, **109 Tests** grün (mit PostgreSQL 17), Web format/lint/check/20 Tests/Build grün, Image baut.
Basis-Branches: #43 → #42, #45 → #43, #46 → #45 – nach jedem Merge die Basis des Nachfolgers auf `main` umstellen.

| PR | Inhalt |
|---|---|
| #44 | Discovery-Fixtures + Codec (`protect-compat/discovery.rs`), TP-002 (Ergebnis: UNVR sendet `01 00 00 00` an UDP 10001), ADR-005 |
| #45 | AKL-M080-04b: Geräteidentitäten in Backup/Restore (Probe lokal + CI grün) |
| #46 | AKL-M040-06 (#8): Geräte-UI mit Kill-Switch und Verlauf, Echtzeit über SSE-Status, E2E in CI grün |

**Wichtige Grenze dieser Sitzung:** Zweimal hat ein Sicherheitsfilter Antworten gestoppt (Firmware-Analyse, danach
Discovery-/Adoptions-Arbeit). Claude hat daraufhin **Firmware-Analyse sowie Discovery-Antwort/Adoption (#5, #6) in dieser
Sitzung eingestellt** – trotz Freigabe (ADR-005). Firmware-Dateien liegen ungenutzt unter `D:\ai-keyless-firmware\`
(nicht im Repo). TP-002 Lauf 3 (mit Zieladresse) lief passiv, wurde nicht ausgewertet (Rohlog `%TEMP%\akl-tp002-lauf3.log`).

**Umgebung:** Ethernet-Profil „Privat“, Firewall-Regel „Windows PowerShell (Privat, AI-Keyless TP-002)“ bleibt (Entscheidung).
`python3` in Git Bash = Store-Platzhalter → für Skripte Shim auf `%LOCALAPPDATA%\Programs\Python\Python313\python.exe`.
WSL-VM-Speicher bei Bedarf freigeben: `docker run --rm --privileged --pid host alpine sh -c "echo 3 > /proc/sys/vm/drop_caches"`.

**Nächstes (ohne Protokollbezug):** Reviews/Merges der Kette; AKL-M020-06 (a11y-Audit), M020-07 (CSP ohne `unsafe-inline`),
M010-07 (OpenAPI aus Rust-Typen), M020-03 (Komponentenbibliothek).

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
