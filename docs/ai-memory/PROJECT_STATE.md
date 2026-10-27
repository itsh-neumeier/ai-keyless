# Projektstand

> Zuletzt aktualisiert: 2026-10-27 (Nacht) · Branch `feat/AKL-M040-07-api-wiring`

## Aktive Meilensteine

| Meilenstein | Status |
|---|---|
| AKL-M000 Recht/Bestand/Machbarkeit | fertig bis auf Hersteller-FAQ (Q-004, zurückgestellt) und Lizenzanfrage an Referenz (nur nach Freigabe) |
| AKL-M010 Projektfundament | fertig; offen: OpenAPI-Generierung (M010-07), Repo-Governance (M010-08) |
| AKL-M020 UI-System | Auth, Kontoverwaltung (deaktivieren, Passwort ändern) und E2E in CI fertig; offen: Komponentenbibliothek (M020-03), Demo-Modus (M020-05), a11y-Audit (M020-06), CSP-Hashes (M020-07) |
| AKL-M030 Contract-Labor | Versionsprofile (M030-01) und Testplan-Vorlage + TP-001-Entwurf (M030-04) fertig; offen: synthetischer Controller (M030-02), Fixtures (M030-03) – beide nur auf Basis belegbarer Hypothesen sinnvoll |
| AKL-M080 Betrieb | Backup/Restore + Wiederherstellungsprobe (CI) fertig; offen: Netzmodus-Bewertung (M080-01), Portainer (M080-02), ARM64 (P3), GPU-Profil (M080-06, Q-014) |
| AKL-M060 Provider/Queue | Provider + SSRF-Schutz, Ollama, OpenAI-kompatibel (Mock), Queue, Worker, Limit, UI fertig; offen: OpenAI-kompatibel mit echtem Server, Cloud-Provider (Kostenrahmen Q-011), aggregierte Statistik |

## Belegter Funktionsumfang (Stand 2026-10-27)

- API mit Authentifizierung: alle Endpunkte außer `/healthz`, `/readyz`, `/api/v1/auth/{state,setup,login}` erfordern eine Sitzung.
- UI-Bereiche mit echten Daten: Übersicht, Geräte (Profile + Kompatibilitätsmatrix), KI-Modelle, Verarbeitung, System, Einstellungen (inkl. Benutzer).
- Verarbeitungs-Queue mit Worker; einziger Job-Typ bisher `vision_probe`.
- Modell-Provider real gemessen: Ollama 0.12.6 + granite3.2-vision (CPU) erkennt das synthetische Testbild; moondream liefert leere Antworten (`docs/compatibility/PROVIDERS.md`).
- **Geräte-Emulation nur synthetisch** (AKL-M040-03/04, Branches `feat/AKL-M040-03-discovery-responder` → `feat/AKL-M040-04-adoption`, noch nicht gemergt): Discovery-Antwort, Adoption, Keepalive/Reconnect `synthetic_verified` gegen `akl-protect-sim`; PRs #51/#52 (CI grün, Merge offen); opt-in-Verdrahtung in `akl-api` auf `feat/AKL-M040-07-api-wiring` (`AKL_DEVICE_EMULATION`, Standard aus); nie gegen echte Protect gelaufen. Keine `native_verified`-Aussage.
- Testumgebung (Stand 2026-10-27 nach Update): UNVR, **UniFi OS 6.0.11, Protect 7.3.70**, 8 × G5 Turret Ultra (FW 5.5.43), Produktivbetrieb; Lab-PC mit T600 (Ollama, 20/41 Schichten GPU, Kaltstart >120 s, warm ~0,5 s) und Coral (in WSL/Container sichtbar, ungetestet).

## Rahmenbedingungen (Interview 2026-10-26)

- Lizenz AGPL-3.0-or-later; Restrisiko Geräteimitation bewusst getragen (privat).
- Zielplattform: Linux-Server/VM und x86_64-NAS; GPU: NVIDIA < 12 GB, Intel/AMD, Coral Edge TPU.
- Provider-Reihenfolge: Ollama → OpenAI-kompatibel lokal → OpenAI Cloud (nach Kostenfreigabe) → weitere Cloud.
- GitHub: Draft-PR #1 erlaubt; Issues noch nicht.

## Gemergt in `main` (Stand 2026-10-27, CI grün)

- #39 TP-000, #40 Kaltstart T600, #41 Netzwerk-Lab (AKL-M080-01), #42 Geräteidentität (AKL-M040-01),
  #43 Lifecycle-Service + Kill-Switch (AKL-M040-02), #44 Discovery-Fixtures/Codec/TP-002/ADR-005 (AKL-M030-03),
  #45 Identitäts-Backup (AKL-M080-04b), #46 Geräte-UI (AKL-M040-06), #47 a11y-Audit (AKL-M020-06),
  #48 CSP ohne `unsafe-inline` (AKL-M020-07).

## Nächstes Entwicklungspaket

1. Merges der Kette #39 → #46 (Reihenfolge in SESSION_HANDOFF).
2. AKL-M020-06/07: Accessibility-Audit (axe in E2E) und CSP ohne `unsafe-inline`.
- **Discovery-Antwort/Adoption (#5, #6) synthetisch umgesetzt (Spätsitzung 2026-10-27)**; Firmware-Analyse weiter eingestellt.
  Verdrahtung (AKL-M040-07) gebaut; nächster Schritt: Merges #51 → #52 → M040-07, Lab-Compose host/macvlan, dann TP-001 mit Freigabe.
3. AKL-M020-03: Komponentenbibliothek (Dialog, Drawer, Toast, Tabs, DataTable).
4. AKL-M010-07: OpenAPI aus Rust-Typen generieren, Drift-Prüfung in CI.
5. Danach M040 (Geräte-Lifecycle) nur mit belastbaren Protokoll-Hypothesen; native Abnahme erst mit Testumgebung (Q-003).
