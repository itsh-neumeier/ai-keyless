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
| PostgreSQL-Migrationen | `synthetic_verified` | `crates/persistence/tests/migrations.rs` gegen postgres:17-alpine (2026-10-26) |
| Gerätezustandsautomat | `implemented` (Unit-Tests) | `crates/core/src/device.rs` – noch ohne Lifecycle-Anbindung |
| Container linux/amd64 | `synthetic_verified` | Image-Build + Smoke-Test 2026-10-26 |
| Container linux/arm64 | `planned` | AKL-M080-03 |
| Portainer-Stack | `planned` | AKL-M080-02 (Compose ist Portainer-kompatibel formuliert, nicht getestet) |

### Profil `ai_key`

| Funktion | Protect-Version | Kamera | Stufe | Anmerkung |
|---|---|---|---|---|
| UDP/LAN-Discovery | – | – | `planned` | Referenz berichtet für 7.3.56 (→ `needs_evidence`) |
| Adoption / Identität / Zertifikate | – | – | `planned` | AKL-M040 |
| Kontrollkanal (UCP4/WebSocket) | – | – | `needs_evidence` | Vertrag unbekannt |
| Keepalive / Reconnect | – | – | `planned` | |
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

## Nachweisformat für `native_verified`

Jeder Eintrag benötigt: Datum, Protect-Version, Konsolenmodell, Kameramodell + Firmware,
AI-Keyless-Commit, Testplan-ID, Methode, redigierte Logs, Readback-Beleg (UI-Neuladen bzw.
API-Readback), offene Risiken. Ablage: `docs/compatibility/evidence/<datum>-<thema>.md`.
