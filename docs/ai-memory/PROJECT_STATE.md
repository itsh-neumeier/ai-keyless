# Projektstand

> Zuletzt aktualisiert: 2026-10-26 · Branch `claude/ai-keyless-unifi-protect-w0ntn0`

## Aktive Meilensteine

| Meilenstein | Status |
|---|---|
| AKL-M000 Recht/Bestand/Machbarkeit | fertig bis auf Hersteller-FAQ (Q-004, zurückgestellt) und Lizenzanfrage an Referenz (nur nach Freigabe) |
| AKL-M010 Projektfundament | fertig; offen: OpenAPI-Generierung (M010-07), Repo-Governance (M010-08) |
| AKL-M020 UI-System | Auth (Setup-Token, Login, Rollen, CSRF, Audit) fertig; offen: Komponentenbibliothek (M020-03), Konto deaktivieren/Passwort ändern (M020-04), Demo-Modus, a11y-Audit, CSP-Hashes, E2E in CI |
| AKL-M030 Contract-Labor | Versionsprofile + konservativer Modus fertig (M030-01); offen: synthetischer Controller (M030-02), Fixtures (M030-03), Testplan-Vorlage (M030-04) |
| AKL-M060 Provider/Queue | Provider + SSRF-Schutz, Ollama, OpenAI-kompatibel (Mock), Queue, Worker, Limit, UI fertig; offen: OpenAI-kompatibel mit echtem Server, Cloud-Provider (Kostenrahmen Q-011), aggregierte Statistik |

## Belegter Funktionsumfang (Stand 2026-10-27)

- API mit Authentifizierung: alle Endpunkte außer `/healthz`, `/readyz`, `/api/v1/auth/{state,setup,login}` erfordern eine Sitzung.
- UI-Bereiche mit echten Daten: Übersicht, Geräte (Profile + Kompatibilitätsmatrix), KI-Modelle, Verarbeitung, System, Einstellungen (inkl. Benutzer).
- Verarbeitungs-Queue mit Worker; einziger Job-Typ bisher `vision_probe`.
- Modell-Provider real gemessen: Ollama 0.12.6 + granite3.2-vision (CPU) erkennt das synthetische Testbild; moondream liefert leere Antworten (`docs/compatibility/PROVIDERS.md`).
- **Keinerlei Protect-Kommunikation implementiert. Keine `native_verified`-Aussage.** Keine Protect-Testumgebung vorhanden (Q-003).

## Rahmenbedingungen (Interview 2026-10-26)

- Lizenz AGPL-3.0-or-later; Restrisiko Geräteimitation bewusst getragen (privat).
- Zielplattform: Linux-Server/VM und x86_64-NAS; GPU: NVIDIA < 12 GB, Intel/AMD, Coral Edge TPU.
- Provider-Reihenfolge: Ollama → OpenAI-kompatibel lokal → OpenAI Cloud (nach Kostenfreigabe) → weitere Cloud.
- GitHub: Draft-PR #1 erlaubt; Issues noch nicht.

## Nächstes Entwicklungspaket

1. AKL-M020-08: Playwright-E2E (Setup, Login, Provider, Queue) in die CI übernehmen.
2. AKL-M080-04: Backup/Restore mit automatisierter Wiederherstellungsprobe.
3. AKL-M030-04: Testplan-Vorlage für native Tests (Voraussetzung, sobald eine Testumgebung existiert).
4. AKL-M080-01: Netzwerkmodus-Bewertung (bridge/host/macvlan) per Messtest.
5. AKL-M020-03/04: Komponentenbibliothek, Konto deaktivieren, Passwort ändern.
