# Entscheidungen

| Datum | Entscheidung | Begründung | Verweis |
|---|---|---|---|
| 2026-10-26 | Clean-Room, kein Referenzcode | Referenzprojekt ohne Lizenz | ADR-002, PROVENANCE.md |
| 2026-10-26 | Rust/Axum/SQLx/PG17 + SvelteKit 3.1/Svelte 5/TS 6/Tailwind 4 | TalkOps-Linie, Versionen verifiziert und gepinnt | ADR-001 |
| 2026-10-26 | UI als statische SPA, ausgeliefert durch Rust-API | ein Container, Auth zentral | ADR-001 |
| 2026-10-26 | Profile `ai_key`/`ai_port` strikt getrennt, expliziter Zustandsautomat | Vermischungs- und Falschaussagerisiko | ADR-003 |
| 2026-10-26 | Admin-Port 8090, Compose bindet standardmäßig nur 127.0.0.1 | Kollision mit berichteter Geräteschnittstelle 8080 vermeiden; sicherer Default | OVERVIEW.md |
| 2026-10-26 | Unbekannter Zustand = `null`, nie geraten | Wahrheitsregel | ADR-003 |
| 2026-10-26 | Weitere Crates erst mit ihrem Meilenstein | keine leeren, ungetesteten Hüllen | ADR-001 |
| 2026-10-26 | Startlimit 12 Jobs/h (Sicherheitsvorgabe, kein Leistungsziel) | Kosten/Last; aus Referenz-Doku als Praxiswert | BACKLOG M060-03 |
| 2026-10-26 | GitHub-Issues noch nicht angelegt; lokaler Backlog + Import-Skript | Veröffentlichung nicht ausdrücklich freigegeben | BACKLOG.md |
| 2026-10-26 | Docker darf für alle Entwicklertätigkeiten genutzt werden | Anweisung des Nutzers | – |
