# Kompatibilitätsmatrix

> Jede Fähigkeitsaussage hat genau eine Evidenzstufe und eine konkrete Version.
> Stufen: `planned` · `implemented` · `synthetic_verified` · `native_verified` · `unsupported` · `needs_evidence`
> **Regel:** HTTP 200 oder grüne Unit-Tests sind niemals `native_verified`.

## Stand AI-Keyless 0.1.0 (2026-10-26)

**Es gibt derzeit keine einzige `native_verified`-Aussage.** AI-Keyless wurde noch nie gegen
ein echtes UniFi-Protect-System getestet.

### Plattform (AI-Keyless selbst)

| Funktion | Stufe | Beleg |
|---|---|---|
| Management-API `/healthz`, `/readyz`, `/api/v1/system/status` | `synthetic_verified` | `apps/api/tests/http.rs`; Compose-Smoke-Test `scripts/smoke-compose.sh` (2026-10-26, Docker 29.9.2, linux/amd64) |
| Authentifizierung (Einrichtung, Login, Rollen, CSRF, Audit) | `synthetic_verified` | `apps/api/tests/auth_flow.rs` (echte PG 17), Playwright-Ablauf, Compose-Smoke-Test (2026-10-27) |
| Verarbeitungs-Queue (Idempotenz, Leases, Backoff, Limit, `uncertain`) | `synthetic_verified` | `crates/persistence/tests/jobs.rs`, `apps/api/tests/jobs_flow.rs`; echter Worker-Lauf mit Ollama 0.12.6 + granite3.2-vision (2026-10-27) |
| Modell-Provider | siehe `PROVIDERS.md` | |
| PostgreSQL-Migrationen | `synthetic_verified` | `crates/persistence/tests/migrations.rs` gegen postgres:17-alpine (2026-10-26) |
| Gerätezustandsautomat | `synthetic_verified` | `crates/core/src/device.rs`; Lifecycle-Service `crates/device-lifecycle/tests/service.rs` (echte PG 17) |
| Container linux/amd64 | `synthetic_verified` | Image-Build + Smoke-Test 2026-10-26 |
| Container linux/arm64 | `planned` | AKL-M080-03 (nachrangig, keine Zielplattform) |
| Backup/Restore inkl. Wiederherstellungsprobe | `synthetic_verified` | `scripts/restore-test.sh` (2026-10-27, lokal; CI) |
| Portainer-Stack | `planned` | AKL-M080-02 (Compose ist Portainer-kompatibel formuliert, nicht getestet) |

### Profil `ai_key`

| Funktion | Protect-Version | Kamera | Stufe | Anmerkung |
|---|---|---|---|---|
| UDP/LAN-Discovery: Antwort auf Suchanfrage | – (nur `akl-protect-sim`) | – | `synthetic_verified` | AI-Keyless 0.1.0, 2026-10-27: `crates/device-lifecycle/tests/discovery.rs` (echte PG 17) – v1/v2-Probe beantwortet, fremde/fehlerhafte Pakete ohne Antwort und ohne Zustandswechsel, Kill-Switch/deaktiviert ⇒ stumm, Rate-Limit je Absender. Gegenüber Protect `needs_evidence` (DISC-H07, TP-001); im Profil-JSON daher unverändert |
| Adoption (eigenes Zertifikat, Controller-Pin vor Token-Übergabe, Token nur als 0600-Datei) | – (nur `akl-protect-sim`) | – | `synthetic_verified` | AI-Keyless 0.1.0, 2026-10-27: `crates/device-lifecycle/tests/adoption.rs` (echte PG 17) – `discovered → pending_adoption → adopted`; falscher Pin/unerreichbar ⇒ Rückweg `→ discovered`, kein Token gesendet/gespeichert; fremde Quelle ohne Handshake; Kill-Switch/Zustand/ungültige Anfrage abgelehnt. Zeilenformat ist synthetischer Platzhalter; gegenüber Protect `needs_evidence` (Q-006, TP-001). Credential-Rotation: `planned` |
| Kontrollkanal (UCP4/WebSocket) | – | – | `needs_evidence` | Vertrag unbekannt (Q-007); synthetischer Platzhalter-Kanal siehe nächste Zeile |
| Keepalive / Reconnect / Neustart-Readback (synthetischer Kanal) | – (nur `akl-protect-sim`) | – | `synthetic_verified` | AI-Keyless 0.1.0, 2026-10-27: `adopted → online`; Controller weg ⇒ `reconnecting → offline`, zurück ⇒ `online`; Dienstneustart ⇒ `reconnecting → online` ohne Neu-Adoption; geändertes Controller-Zertifikat ⇒ `error`, Token nicht gesendet; `→ disabled` beendet den Kanal. Gegenüber Protect `needs_evidence` |
| On-demand-Beschreibung | – | – | `planned` | Referenz: 7.3.60, G5 Flex |
| Automatische Beschreibung + Persistenz | – | – | `planned` | Referenz: 7.3.60, G5 Flex (1 Event) |
| Find Anything (nativ) | – | – | `needs_evidence` | Such-Vertrag offen (CLIP vs. E5) |
| AI Alerts | – | – | `needs_evidence` | |
| Speech-to-Text, LPR, Gesicht, Enhancement | – | – | `needs_evidence` | |

### Profil `ai_port`

| Funktion | Protect-Version | Kamera | Stufe | Anmerkung |
|---|---|---|---|---|
| Adoption / Pairing | – | – | `needs_evidence` | Referenz: Kandidat 7.3.60, G3 |
| Stream-Verarbeitung, Smart Zones, Detektionen | – | – | `needs_evidence` | AKL-M110 |

## Berichte Dritter (nicht unsere Evidenz)

Siehe `REFERENCE-ANALYSIS.md`. Diese Aussagen werden **niemals** in die Tabellen oben
übernommen, sondern dienen nur als Testhypothesen.

## Maschinenlesbare Quelle

Die Profile pro Protect-Version liegen in `crates/protect-compat/profiles/{ai_key,ai_port}.json`
und werden in der UI unter *Geräte* angezeigt. Freigaberegel (`Resolution::decide`):
`native_verified` → aktiv; `synthetic_verified` → nur mit ausdrücklichem Opt-in; alles andere
→ inaktiv; unbekannte oder nicht lesbare Version → konservativer Modus (nichts aktiv, Warnung).
Ein `native_verified`-Eintrag ohne Verweis auf `evidence/…` wird beim Laden abgelehnt; ein Test
stellt sicher, dass es derzeit keinen einzigen gibt. Änderungen an den JSON-Dateien und an dieser
Matrix erfolgen immer gemeinsam.

## Nachweisformat für `native_verified`

Jeder Eintrag benötigt: Datum, Protect-Version, Konsolenmodell, Kameramodell + Firmware,
AI-Keyless-Commit, Testplan-ID, Methode, redigierte Logs, Readback-Beleg (UI-Neuladen bzw.
API-Readback), offene Risiken. Ablage: `docs/compatibility/evidence/<datum>-<thema>.md`.
