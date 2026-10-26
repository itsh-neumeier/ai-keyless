# ADR-001: Architektur und Technologie-Stack

- **Status:** Akzeptiert · **Datum:** 2026-10-26 · **Meilenstein:** AKL-M010

## Kontext

AI-Keyless soll ressourcenschonend auf NAS/Kleinservern (amd64/arm64) laufen, sicherheitskritische
Netzwerkprotokolle sprechen und eine moderne Admin-UI bieten. Vorgabe: TalkOps-Linie
(Rust/Axum/SQLx/PostgreSQL, SvelteKit/Svelte 5/TypeScript/Tailwind).

## Entscheidung

| Bereich | Wahl | Verifizierte/gelockte Version (2026-10-26) |
|---|---|---|
| Backend | Rust stable, Tokio, Axum, serde, tracing | rustc/cargo 1.97.0, axum 0.8.9, tokio 1.54, tower-http 0.7.1 |
| Persistenz | PostgreSQL + SQLx-Migrationen | sqlx 0.9.0, postgres 17 |
| Frontend | SvelteKit 3 + Svelte 5 + TypeScript + Tailwind CSS 4 | @sveltejs/kit 3.1.0 (dist-tag `latest`), svelte 5.57.3, vite 8.3.5, typescript 6.0.3, tailwindcss 4.3.3 |
| Paketmanager Web | pnpm (exakt gepinnte Versionen, Lockfile) | pnpm 10.28.0 |
| Auslieferung UI | **Statische SPA** (`adapter-static`, Fallback `index.html`), ausgeliefert von der Rust-API | – |
| Echtzeit | Server-Sent Events (`/api/v1/system/status/stream`), Server sendet nur Änderungen | – |
| API-Vertrag | Rust-Typen in `crates/core` als Quelle; `docs/api/openapi.yaml`; Laufzeitvalidierung im Client | – |
| Container | Multi-Stage-Build, Laufzeit `distroless/cc-debian12:nonroot`, read-only, `cap_drop: ALL` | Image ≈ 52 MB |

**Monorepo-Zuschnitt (Stand M010):** `apps/api`, `apps/web`, `crates/core`, `crates/persistence`.
Die weiteren Crates aus dem Zielbild (`protect-compat`, `device-lifecycle`, `event-engine`,
`providers`, `observability`) werden **erst mit ihrem Meilenstein** angelegt, um leere
Hüllen ohne Tests zu vermeiden.

**SvelteKit-3-Besonderheiten:** Konfiguration im Vite-Plugin (`sveltekit({...})`, keine
`svelte.config.js`); `$lib` ist entfallen → Node-Subpath-Imports `#lib/*` (in `package.json`
`imports`) mit expliziten Dateiendungen; `tsconfig.json` erweitert `$app/tsconfig`.
TypeScript bleibt auf 6.0.x, da Kit 3.1 `typescript ^6` als Peer fordert (TS 7 wäre ohne ADR
ein unzulässiger Sprung).

**Warum SPA statt SSR-Node-Server:** ein Prozess/Container weniger, kleinere Angriffsfläche,
Authentifizierung/CSRF zentral in Rust, keine Node-Laufzeit im Produktivimage. Nachteil: kein SSR –
für eine authentifizierte Admin-Oberfläche im LAN irrelevant.

**ML-Inferenz:** nicht in Rust neu implementiert. Provider-Schnittstellen (Ollama,
OpenAI-kompatibel, OpenAI Responses) in `crates/providers` (M060); optional separater
Python/ONNX-Worker in `workers/inference` nur bei Bedarf. CUDA/Coral sind optionale Backends.

## Alternativen

- *SvelteKit adapter-node + separater Node-Container* – verworfen (zusätzlicher Prozess, doppelte Auth).
- *WebSockets statt SSE* – für Status unnötig; WebSockets bleiben für spätere bidirektionale Fälle offen.
- *SQLite* – verworfen: Vorgabe PostgreSQL, spätere pgvector-Option.
- *TypeScript 7* – verworfen bis Kit-Peer-Unterstützung.

## Konsequenzen

- Ein Image (`ai-keyless`) + PostgreSQL. API startet auch ohne erreichbare DB und meldet
  ehrlich `unavailable`/`degraded`; Migrationen laufen mit Backoff im Hintergrund.
- TypeScript-Typen müssen bei API-Änderungen manuell mitgezogen werden, bis ein
  Generator (Backlog AKL-M010-07) eingeführt ist; Laufzeitvalidierung fängt Abweichungen ab.

## Überprüfung

Bei Einführung von Authentifizierung (M020) und der ersten bidirektionalen Echtzeitfunktion.
