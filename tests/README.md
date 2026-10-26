# Tests (repositoryweit)

- `fixtures/` – **ausschließlich künstliche** Testdaten (keine echten Kamerabilder, Personen, Kennzeichen, Seriennummern).
- `contracts/` – versionierte Protect-Contract-Tests mit Evidenzstufe je Fixture (AKL-M030).
- `integration/` – Mehrkomponententests (synthetischer Controller ↔ AI-Keyless).
- `e2e/` – Playwright-Tests der Admin-UI (AKL-M020-08).

Crate-lokale Tests liegen in `crates/*/tests` und `apps/api/tests`. Siehe `docs/testing/STRATEGY.md`.
