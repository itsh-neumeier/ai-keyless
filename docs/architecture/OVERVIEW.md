# Architekturübersicht

> Stand: 2026-10-26 (AKL-M010). Details und Begründungen: ADR-001…003.

## Komponenten

```text
          Browser (Admin)                       UniFi Protect (Controller, autorisiert)
               │ HTTPS/HTTP (LAN)                        ▲   │
               ▼                                         │   │  (ab M040: Discovery, Adoption,
 ┌──────────────────────────────┐                        │   │   Kontrollkanal, Callbacks)
 │ ai-keyless (ein Container)   │                        │   ▼
 │  apps/api  (Axum)            │◄──── protect-compat ───┘ device-lifecycle   [geplant]
 │   ├─ REST /api/v1/*          │
 │   ├─ SSE  /api/v1/.../stream │      event-engine ──► providers ──► Ollama / OpenAI-kompatibel
 │   ├─ /healthz /readyz        │      [geplant M060]                 [extern, opt-in]
 │   └─ static SPA (apps/web)   │
 └──────────────┬───────────────┘
                │ SQLx (TLS optional)
                ▼
         PostgreSQL 17 (Volume)
```

## Crates und Grenzen

| Crate/App | Verantwortung | Darf abhängen von | Status |
|---|---|---|---|
| `crates/core` | Domänenmodelle, Zustandsautomat, Evidenz, API-Statusverträge; **kein I/O** | serde, thiserror | vorhanden |
| `crates/persistence` | Pool, Migrationen, Repositories | core, sqlx | vorhanden |
| `apps/api` | HTTP/SSE, Konfiguration, Auslieferung der SPA, Security-Header | core, persistence | vorhanden |
| `apps/web` | SvelteKit-SPA, Design-System, i18n | nur HTTP-API | vorhanden |
| `crates/protect-compat` | Versionsprofile (`profiles/*.json`, eingebettet und beim Laden validiert), Fähigkeitsfreigabe nach Evidenz, konservativer Modus; später Protokolladapter | core | Profile ✓ (M030-01), Adapter M030/M040 |
| `crates/device-lifecycle` | Identität je Profil (`identity.rs`, Dateispeicher unter `AKL_DEVICE_DIR`), Lifecycle-Service mit Kill-Switch (`service.rs`, einziger Weg für Profilanlage und Zustandsänderungen); später Discovery, Adoption, Keepalive | core, protect-compat, persistence | Identität ✓ (M040-01), Service ✓ (M040-02), Rest M040 |
| Queue/Worker | Persistente Queue (`persistence/src/jobs.rs`), Zustände (`core/src/job.rs`), Worker im API-Prozess (`apps/api/src/worker.rs`); eigenes Crate `event-engine`, sobald Protect-Callbacks dazukommen (M070) | core, persistence, providers | ✓ M060-01/03 |
| `crates/providers` | Ollama/OpenAI-kompatibel, SSRF-Richtlinie (`policy.rs`), synthetisches Testbild | – | ✓ M060-02/05 |
| `crates/observability` | Metriken, Health-Aggregation | core | M060/M080 |

Regel: Abhängigkeiten zeigen nur „nach innen“ (zu `core`). `core` kennt weder HTTP noch DB.

## Laufzeitverhalten

- **Start:** Konfiguration aus Umgebungsvariablen/Secret-Dateien (`AKL_DATABASE_URL_FILE`).
  DB-Pool wird lazy erzeugt; Migrationen laufen im Hintergrund mit exponentiellem Backoff (max. 60 s).
- **Health:** `/healthz` = Prozess lebt; `/readyz` = DB erreichbar und migriert (oder bewusst nicht konfiguriert).
- **Status:** `/api/v1/system/status` aggregiert DB-Zustand, Schema-Version und gespeicherte
  Geräteprofilzustände. Ohne Profil → `state: null`.
- **Echtzeit:** SSE-Stream prüft serverseitig alle 5 s und sendet nur bei Änderung; Keep-Alive-Kommentare halten die Verbindung.
- **Logging:** strukturiert (`AKL_LOG_FORMAT=json`), `x-request-id` als `correlation_id`; DB-URL wird redigiert.
- **Shutdown:** SIGTERM/SIGINT → Graceful Shutdown.

## Ports

| Port | Zweck | Standard-Bindung |
|---|---|---|
| 8090/tcp | Admin-UI + Management-API | Compose: nur `127.0.0.1` |
| (8080/tcp) | AI-Key-Geräteschnittstelle laut Referenzbericht | **nicht belegt**, M040 |
| (443/tcp) | AI-Port-Instanz laut Referenzbericht | **nicht belegt**, M110 |

8090 wurde gewählt, um nicht mit der (berichteten) Gerätschnittstelle 8080 zu kollidieren.
