# ADR-002: Lizenz- und Clean-Room-Strategie

- **Status:** Akzeptiert, Punkt 3 (Firmware-Images) teilweise ersetzt durch **ADR-005** (2026-10-27) · **Datum:** 2026-10-26 · **Meilenstein:** AKL-M000

## Kontext

Das technische Referenzprojekt `Olli0103/unifi-ai-key-emulator` (Commit `4c71b73`) besitzt
**keine Lizenz**. Damit ist jede Übernahme, Portierung oder Weiterveröffentlichung seines Codes
ohne Genehmigung unzulässig. Zudem betrifft das Projekt proprietäre Herstellerprotokolle.

## Entscheidung

1. **Eigenständige Implementierung** in Rust/TypeScript im neuen Repository.
2. **Erlaubte Quellen:** öffentliche Dokumentation (als Faktenquelle), öffentlich freigegebene
   Schnittstellen (z. B. Protect Integration API), Black-Box-Beobachtung **eigener bzw.
   ausdrücklich autorisierter** Systeme.
3. **Verbotene Quellen ohne dokumentierte Freigabe:** fremder Quellcode (auch nicht „nur
   angeschaut und nachgetippt“), Firmware-Images, extrahierte Schlüssel/Zertifikate, Modellgewichte,
   Webassets/Icons/CSS des Herstellers.
4. Jede Quelle wird **vor** Nutzung in `docs/legal/PROVENANCE.md` eingetragen.
5. Agenten/Entwickler, die künftig Referenzcode lesen sollten (nach Freigabe), arbeiten getrennt
   von denen, die die Clean-Room-Implementierung schreiben; das wird im PR vermerkt.
6. Keine Umgehung von Lizenzprüfungen, Zugriffssicherungen oder fremder Infrastruktur.

## Migration bei späterer Lizenzfreigabe

Erst reproduzierbare Bestands- und Vertragstests gegen das freigegebene Artefakt, dann
schrittweise Migration hinter Adaptern (`crates/protect-compat`). Eine bestehende Adoption darf
dabei nicht regressieren. Neuer ADR erforderlich.

## Konsequenzen

- Langsamerer Start, dafür saubere Provenienz und Veröffentlichbarkeit.
- Protokollwissen entsteht in `tests/contracts` als versionierte, synthetische Fixtures.
