# Backlog

> **Generiert** aus `backlog.json` via `scripts/gen-backlog-md.py` – nicht von Hand bearbeiten.
> Stand: 2026-10-26. Nach Anlage der GitHub-Issues (`scripts/import-backlog.sh`) sind diese führend.

**Definition of Done (jedes Issue):** Funktion umgesetzt; Unit-, Negativ- und Integrationstests; Doku; Sicherheitsprüfung; Migrations-/Rollback-Betrachtung; Versionsevidenz; PR-Review; AI-Memory aktualisiert.

Prioritäten: P0 kritisch · P1 hoch · P2 mittel · P3 später.

## AKL-M000 – Recht, Bestand, Machbarkeit

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M000-01 | Lizenz-/Provenienzprüfung Referenzprojekt | P0 | ✅ erledigt | – | PROVENANCE.md mit Commit, Lizenzstatus, Methode<br>Go/No-Go-Gates dokumentiert |
| AKL-M000-02 | Bestandsbericht bestätigte vs. unbestätigte Funktionen | P0 | ✅ erledigt | – | REFERENCE-ANALYSIS.md<br>Matrix ohne übernommene Fremdevidenz |
| AKL-M000-03 | Offizielle AI-Key-FAQ manuell sichten (help.ui.com nicht abrufbar) | P1 | ⛔ blockiert | – | Funktionsliste, Limits, Mindestversionen in OPEN_QUESTIONS/Matrix erfasst |
| AKL-M000-04 | Lizenzanfrage an Referenz-Maintainer (nur nach Freigabe, durch Projektverantwortlichen) | P2 | ⛔ blockiert | – | Antwort dokumentiert in PROVENANCE.md |
| AKL-M000-05 | Projektlizenz für AI-Keyless festlegen | P0 | ⛔ blockiert | – | LICENSE-Datei<br>Cargo/package.json aktualisiert<br>ADR |
| AKL-M000-06 | Rechtliche Einschätzung Geräteimitation/Interoperabilität (DE/EU) | P1 | ⛔ blockiert | – | Kurzgutachten oder bewusste Risikoentscheidung des Projektverantwortlichen dokumentiert |

## AKL-M010 – Projektfundament

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M010-01 | Monorepo, Rust-Workspace, SvelteKit-App, Basisdokumente | P0 | ✅ erledigt | – | cargo build/test grün<br>pnpm check/lint/test/build grün |
| AKL-M010-02 | API /healthz, /readyz, Systemstatus + SSE | P0 | ✅ erledigt | AKL-M010-01 | HTTP-Vertragstests<br>kein erfundener Gerätezustand |
| AKL-M010-03 | PostgreSQL-Grundschema + Migrationstest | P0 | ✅ erledigt | AKL-M010-01 | Migration idempotent gegen echte DB<br>Constraints getestet |
| AKL-M010-04 | Dockerfile, Compose, Secrets, Smoke-Test | P0 | ✅ erledigt | AKL-M010-02 | Image nonroot/read-only<br>Smoke-Test grün |
| AKL-M010-05 | Basis-CI (Rust, Web, Security, Container) | P0 | 🔄 in Arbeit | AKL-M010-01 | Workflow vorhanden<br>erster grüner Lauf auf GitHub |
| AKL-M010-06 | Ersten CI-Lauf auf GitHub verifizieren und Fehler beheben | P1 | ⬜ offen | AKL-M010-05 | Alle Jobs grün auf PR |
| AKL-M010-07 | OpenAPI aus Rust-Typen generieren + TS-Client-Typen prüfen | P2 | ⬜ offen | AKL-M010-02 | CI schlägt fehl bei Drift zwischen Rust, OpenAPI und TS |
| AKL-M010-08 | Branch-Schutz, PR-Template, CODEOWNERS, Issue-Templates | P2 | ⬜ offen | AKL-M010-05 | Review-Pflicht auf main |

## AKL-M020 – UniFi-inspiriertes UI-System

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M020-01 | Setup-Modus mit Einmal-Token + Admin-Konto (Argon2id) | P0 | ⬜ offen | AKL-M010-02 | Token einmalig, zeitlich begrenzt<br>Negativtests: Wiederverwendung, Ablauf |
| AKL-M020-02 | Sessions, Rollen Admin/Operator/Viewer, CSRF, Audit-Log | P0 | ⬜ offen | AKL-M020-01 | HttpOnly/SameSite-Cookies<br>CSRF-Negativtests<br>Audit-Einträge |
| AKL-M020-03 | Komponentenbibliothek: Button, Input, Select, Switch, Tabs, Dialog, Drawer, Toast, Menu, DataTable | P1 | ⬜ offen | – | Tastaturbedienung<br>Komponententests |
| AKL-M020-04 | Login-/Setup-Screens und Nutzerverwaltung | P1 | ⬜ offen | AKL-M020-02 | E2E mit Playwright |
| AKL-M020-05 | Getrennter Demo-Modus mit Banner | P2 | ⬜ offen | AKL-M020-03 | Demo-Daten im Normalbuild technisch ausgeschlossen (Test) |
| AKL-M020-06 | Accessibility-Audit (axe) + Kontrastprüfung in CI | P1 | ⬜ offen | AKL-M020-03 | 0 kritische axe-Befunde |
| AKL-M020-07 | CSP ohne unsafe-inline (Hashes) | P2 | ⬜ offen | – | CSP-Test<br>UI funktioniert |
| AKL-M020-08 | Playwright-E2E + visuelle Regression in CI | P2 | ⬜ offen | AKL-M010-05 | Screenshots Dark/Light/Mobil |

## AKL-M030 – Protect-Contract-Labor

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M030-01 | crate protect-compat: Versionsprofile + konservativer Modus | P0 | ⬜ offen | AKL-M010-01 | Unbekannte Version → konservativ + Warnung (Test) |
| AKL-M030-02 | Synthetischer Controller (TLS, Pinning, Fehlerfälle) als Testharness | P0 | ⬜ offen | AKL-M030-01 | Deterministische Tests ohne echtes Material |
| AKL-M030-03 | Contract-Fixtures Discovery/Adoption/Callback als needs_evidence-Hypothesen | P0 | ⬜ offen | AKL-M030-02 | Jede Fixture mit Quelle + Evidenzstufe |
| AKL-M030-04 | Native Testplan-Vorlage + Freigabeprozess für Live-Tests | P1 | ⬜ offen | – | Vorlage in docs/testing<br>Freigabe durch Projektverantwortlichen |

## AKL-M040 – AI-Key-Geräte-Lifecycle

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M040-01 | Persistente Geräteidentität + Schlüssel/Zertifikate (Secret-Store) | P0 | ⬜ offen | AKL-M030-03 | Identität überlebt Neustart<br>Schlüssel nie in DB/Logs |
| AKL-M040-02 | Lifecycle-Service mit Zustandsautomat + Transition-Log | P0 | ⬜ offen | AKL-M040-01 | Unzulässige Übergänge abgelehnt<br>Kill-Switch |
| AKL-M040-03 | Discovery (UDP) gegen synthetischen Controller | P0 | ⬜ offen | AKL-M040-02, AKL-M080-01 | synthetic_verified |
| AKL-M040-04 | Adoption, Credential-Rotation, Keepalive, Reconnect (synthetisch) | P0 | ⬜ offen | AKL-M040-03 | synthetic_verified inkl. Restart |
| AKL-M040-05 | Nativer Akzeptanztest Adoption (autorisierte Umgebung) | P0 | ⛔ blockiert | AKL-M040-04, AKL-M030-04, AKL-M000-06 | native_verified-Nachweis gemäß MATRIX.md |
| AKL-M040-06 | UI Geräte: Profile, Zustand, Evidenz, Recovery-Assistent | P1 | ⬜ offen | AKL-M040-02, AKL-M020-03 | Echtzeit per SSE<br>keine vorgetäuschten Zustände |

## AKL-M050 – Protect-Kamerainventar

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M050-01 | Lesender Kamera-Import (Integration API), Eligibility, Scoping | P1 | ⬜ offen | AKL-M040-04 | Nur lesend<br>ungültiges Inventar → sicher deaktiviert |
| AKL-M050-02 | UI Kameras (Tabelle/Kacheln, Freigabe pro Kamera/Gruppe) | P1 | ⬜ offen | AKL-M050-01 | Änderungen nur lokal |

## AKL-M060 – KI-Provider und Event-Engine

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M060-01 | Persistente Job-Queue mit Zuständen, Idempotenz, Backoff, Dead-Letter | P0 | ⬜ offen | AKL-M010-03 | Crash-Recovery-Test<br>uncertain-Zustand |
| AKL-M060-02 | Provider-Abstraktion (Ollama, OpenAI-kompatibel, Responses) + SSRF-Schutz | P0 | ⬜ offen | – | SSRF-Negativtests<br>Testaufruf nur nach Freigabe bei kostenpflichtigen APIs |
| AKL-M060-03 | Rate-/Kostenlimits (Start: 12 Jobs/h global), Medienlimits | P0 | ⬜ offen | AKL-M060-01 | Limit dauerhaft über Neustart |
| AKL-M060-04 | UI KI-Modelle + Verarbeitung | P1 | ⬜ offen | AKL-M060-01, AKL-M060-02 | Latenz/Fehlerrate aus echten Messungen |

## AKL-M070 – Native Ereignisbeschreibung

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M070-01 | Caption-Callback-Vertrag (synthetisch) + Readback-Prüfung | P0 | ⬜ offen | AKL-M060-03, AKL-M040-04 | Status http_accepted ≠ persisted |
| AKL-M070-02 | Native Ereignisbeschreibung on-demand/automatisch (autorisiert) | P0 | ⛔ blockiert | AKL-M070-01, AKL-M040-05 | native_verified pro Version und Kamera |

## AKL-M080 – Produktiver Linux-/Docker-Betrieb

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M080-01 | Netzwerkmodi bewerten (bridge/host/macvlan) für Discovery | P0 | ⬜ offen | – | Messbarer Test pro Modus<br>Empfehlung in NETWORKING.md |
| AKL-M080-02 | Portainer-Stack testen | P1 | ⬜ offen | AKL-M010-04 | Deployment-Protokoll |
| AKL-M080-03 | ARM64-Image bauen und testen | P1 | ⬜ offen | AKL-M010-04 | Smoke-Test auf arm64 |
| AKL-M080-04 | Backup/Restore + Upgrade/Rollback mit Wiederherstellungsprobe | P1 | ⬜ offen | AKL-M010-03 | Restore-Test automatisiert |
| AKL-M080-05 | 72-h-Endurance- und Lasttest | P2 | ⬜ offen | AKL-M070-02 | Bericht mit Metriken |

## AKL-M090 – Native Suche

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M090-01 | Such-Vertrag klären (CLIP vs. E5) – needs_evidence | P2 | ⬜ offen | AKL-M030-03 | Testplan + Ergebnis |

## AKL-M100 – Erweiterte Funktionen

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M100-01 | AI Alerts, STT, LPR, Gesicht, Enhancement – je eigenes Issue mit DSFA | P3 | ⬜ offen | AKL-M070-02 | Datenschutzprüfung pro Funktion |

## AKL-M110 – AI-Port-Profil

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M110-01 | AI-Port-Profil: eigene Identität, Pairing-Vertrag, Streams | P3 | ⬜ offen | AKL-M040-05 | Getrennt von ai_key (Test) |

## AKL-M120 – Releasequalität

| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |
|---|---|---|---|---|---|
| AKL-M120-01 | Stable-Release-Prozess, GHCR nur nach manueller Freigabe | P2 | ⬜ offen | AKL-M000-05 | Kein automatisches Publish |
| AKL-M120-02 | Lizenzinventur Abhängigkeiten (cargo-deny, license-checker) | P2 | ⬜ offen | – | CI-Prüfung |
