# AGENTS.md – Zuständigkeiten und Regeln für Agenten/Subagents

## Rollen

| Rolle | Zuständig für | Dateigrenzen |
|---|---|---|
| Lead (Hauptsitzung) | Planung, ADRs, Integration, Reviews, AI-Memory, Berichte | alle |
| Backend-Agent | Rust-Crates, Migrationen, API | `apps/api`, `crates/*`, `crates/persistence/migrations` |
| Frontend-Agent | UI, Designsystem, i18n | `apps/web` |
| Contract-/Test-Agent | Fixtures, Contract-/Integrations-/E2E-Tests | `tests/`, `crates/*/tests`, `apps/web/**/*.test.ts` |
| Security-Reviewer | Threat-Model, Reviews, Secret-/Dependency-Scan | `docs/security`, Review-Kommentare |
| Doku-Agent | Betriebs-/Nutzerdoku | `docs/`, `README.md` |

Subagents erhalten **klar abgegrenzte** Aufgaben mit Ziel, Dateigrenzen und Akzeptanzkriterien und
liefern Ergebnis + Testnachweis zurück. Sie committen nicht eigenständig auf `main`.

## Clean-Room-Trennung (ADR-002)

Agenten, die (nach Lizenzfreigabe) Referenzcode lesen, dürfen nicht dieselbe Komponente
implementieren. Ohne Freigabe liest **kein** Agent Referenzquellcode.

## Arbeitsweise

- Ein Issue = ein Branch (`feat/AKL-M020-01-setup-token`, `fix/AKL-M010-fix1-…`) = ein kleiner PR.
- Commit-/PR-Titel beginnen mit der Issue-ID, z. B. `AKL-M020-01: Setup-Token`.
- Keine Refactorings außerhalb des Issues.
- Vor dem Push: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, Web: format/lint/check/test/build.
  Bei DB-Änderungen zusätzlich DB-Tests gegen echte PostgreSQL (Docker).
- Neue Migrationen nur additiv; Rollback-Betrachtung im PR-Text.

## Review-Regeln

- Jeder PR: mindestens ein Review (menschlich oder Security-Reviewer-Agent + Mensch für Merge).
- Checkliste: Tests inkl. Negativfälle · keine Secrets · keine Fremdassets/-code · Evidenzstufen
  korrekt · Doku/AI-Memory aktualisiert · UI zeigt keinen unbelegten Erfolg.

## Tests

- Standard der CI: synthetische Tests. Echte Protect-Tests nur mit freigegebenem Testplan
  (`docs/testing/`), Ergebnisse als `docs/compatibility/evidence/*.md`.
- Keine echten Kamerabilder/Personendaten in Fixtures (`tests/fixtures` nur künstlich).
