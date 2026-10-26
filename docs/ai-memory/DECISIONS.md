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
| 2026-10-26 | **Lizenz AGPL-3.0-or-later**; Quellcode-Link (AGPL §13) über API `source_url` + UI | Interview mit Projektverantwortlichem | ADR-004 |
| 2026-10-26 | Rechtliches Restrisiko der Geräteimitation wird bewusst getragen (privat, nur eigene Geräte) | Interview | docs/legal/RISK-ACCEPTANCE.md |
| 2026-10-26 | Noch keine Testumgebung → nur synthetische Entwicklung; native Tests blockiert | Interview (Q-003) | – |
| 2026-10-26 | GitHub: Draft-PR erlaubt; Issues/Meilensteine noch **nicht** anlegen | Interview (Q-005) | – |
| 2026-10-26 | Zielplattformen: Linux-Server/VM und x86_64-NAS; ARM64 nachrangig (P3) | Interview | BACKLOG M080 |
| 2026-10-26 | Provider-Reihenfolge: Ollama lokal → OpenAI-kompatibel lokal → OpenAI Cloud (Opt-in) → weitere Cloud | Interview (Q-011) | BACKLOG M060-05…08 |
| 2026-10-26 | Hardware: NVIDIA < 12 GB, Intel/AMD-GPU, Coral Edge TPU; Coral nur für Objekterkennung, nicht für Captions | Interview; Edge TPU führt nur quantisierte TFLite-Modelle aus | BACKLOG M080-06, M100-02 |
