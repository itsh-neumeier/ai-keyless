# AI-Keyless

**Inoffizieller, unabhängiger, self-hosted AI-Prozessor für UniFi Protect.**
AI-Keyless ist **nicht** von Ubiquiti entwickelt, zertifiziert oder unterstützt. „UniFi“, „UniFi
Protect“, „AI Key“ und „AI Port“ sind Marken ihrer Inhaber und werden nur beschreibend verwendet.

> ⚠️ **Status 0.1.0 – Projektfundament.** Es ist noch **keine** Protect-Kommunikation implementiert
> und keine Funktion gegen ein echtes Protect-System verifiziert (`native_verified`: 0).
> Siehe [Kompatibilitätsmatrix](docs/compatibility/MATRIX.md).

## Ziel

Ein sauber implementierter, getesteter **AI-Key-Kompatibilitätsmodus**, der von einem
autorisierten UniFi-Protect-System als Gerät erkannt, adoptiert und für **belegte** native
KI-Funktionen genutzt werden kann – später zusätzlich ein getrennter **AI-Port-Modus**.
Anbieterunabhängige Modell-Provider (lokal oder entfernt), Docker-Betrieb, eine
Protect-inspirierte, aber eigenständige Admin-Oberfläche.

## Was heute funktioniert

| Bereich | Stand |
|---|---|
| Management-API (Rust/Axum): Health, Readiness, Status, SSE-Stream | synthetisch verifiziert |
| PostgreSQL-Schema + Migrationen | gegen PostgreSQL 17 getestet |
| Web-UI (SvelteKit 3/Svelte 5/Tailwind 4): App-Shell, Übersicht mit Live-Status, Dark/Light, DE/EN | gebaut, per Playwright geprüft |
| Container (distroless, nonroot, read-only) + Compose | Smoke-Test grün (linux/amd64) |

## Schnellstart (Entwicklung)

```bash
scripts/init-secrets.sh && cp deploy/compose/.env.example deploy/compose/.env
scripts/docker-build.sh
docker compose -f deploy/compose/compose.yaml --env-file deploy/compose/.env up -d
open http://127.0.0.1:8090
```

Entwicklerbefehle: siehe [CLAUDE.md](CLAUDE.md). Betrieb: [docs/operations](docs/operations/README.md).

## Dokumentation

- Architektur: [OVERVIEW](docs/architecture/OVERVIEW.md), [ADRs](docs/adr/README.md)
- Recht/Herkunft: [PROVENANCE](docs/legal/PROVENANCE.md) (Clean-Room, ADR-002)
- Sicherheit: [THREAT-MODEL](docs/security/THREAT-MODEL.md)
- UI: [UI-DESIGN-SYSTEM](docs/ux/UI-DESIGN-SYSTEM.md)
- Roadmap: [BACKLOG](docs/roadmap/BACKLOG.md)
- Projektgedächtnis: [docs/ai-memory](docs/ai-memory/PROJECT_STATE.md)

## Lizenz

Noch nicht festgelegt (siehe [PROVENANCE](docs/legal/PROVENANCE.md) §4). Bis dahin alle Rechte vorbehalten.

## Rechtlicher Hinweis

Nur mit eigenen bzw. ausdrücklich autorisierten Geräten, Konten und Netzwerken verwenden.
AI-Keyless umgeht keine Lizenzprüfungen oder Zugriffssicherungen.
