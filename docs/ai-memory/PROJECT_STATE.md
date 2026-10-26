# Projektstand

> Zuletzt aktualisiert: 2026-10-26 · Branch `claude/ai-keyless-unifi-protect-w0ntn0`

## Aktive Meilensteine

| Meilenstein | Status |
|---|---|
| AKL-M000 Recht/Bestand/Machbarkeit | Bericht, Provenienz, Lizenz (AGPL-3.0-or-later, ADR-004) und Risikoentscheidung fertig; zurückgestellt: Hersteller-FAQ (Q-004), Lizenzanfrage an Referenz (nur nach Freigabe) |
| AKL-M010 Projektfundament | Kern fertig und lokal verifiziert; Draft-PR für ersten CI-Lauf geöffnet (M010-06); offen: OpenAPI-Generierung (M010-07), Repo-Governance (M010-08) |
| AKL-M020 UI-System | App-Shell, Tokens, Dark/Light, i18n, Platzhalter vorgezogen; Auth und Komponentenbibliothek offen |

## Belegter Funktionsumfang (Version 0.1.0)

- API: `/healthz`, `/readyz`, `/api/v1/system/status`, `/api/v1/system/status/stream` (SSE) – `synthetic_verified`.
- PostgreSQL-Schema (Geräteprofile, Zustandsübergänge, Audit-Log) – Migrationstests gegen PG 17 grün.
- Web-App: Übersicht mit echtem Live-Status, System (Unofficial), Einstellungen (Theme/Sprache), Platzhalter für 7 Bereiche.
- Container: distroless/nonroot/read-only, ≈52 MB, Compose-Smoke-Test grün (linux/amd64).
- **Keinerlei Protect-Kommunikation implementiert. Keine `native_verified`-Aussage.**

## Rahmenbedingungen (Interview 2026-10-26)

- Zielplattform: Linux-Server/VM und x86_64-NAS; GPU: NVIDIA < 12 GB, Intel/AMD, Coral Edge TPU.
- Keine Protect-Testumgebung → ausschließlich synthetische Entwicklung bis auf Weiteres.
- Provider zuerst: Ollama lokal.

## Nächstes Entwicklungspaket

1. AKL-M010-06: CI im Draft-PR grün bekommen.
2. AKL-M020-01/02: Setup-Token, Admin-Login, Sessions, CSRF, Audit (Voraussetzung für jede schreibende Funktion).
3. AKL-M030-01/02: `crates/protect-compat` + synthetischer Controller (parallel möglich, Schnittstelle: `core::device`).
4. AKL-M080-01: Netzwerkmodus-Bewertung (Voraussetzung für Discovery).
5. Danach AKL-M060-02/05: Provider-Abstraktion + Ollama (lokal, GPU < 12 GB), unabhängig von Protect testbar.
