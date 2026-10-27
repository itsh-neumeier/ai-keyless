# Teststrategie

| Ebene | Werkzeug | Ort | In CI |
|---|---|---|---|
| Unit (Domäne) | `cargo test` | `crates/*/src` | ✓ |
| HTTP-Vertrag | `tower::ServiceExt::oneshot` | `apps/api/tests` | ✓ |
| DB/Migrationen | echte PostgreSQL 17 (Docker/Service) | `crates/persistence/tests` | ✓ (`AKL_REQUIRE_DB_TESTS=1` verhindert stilles Überspringen) |
| Frontend-Unit | vitest | `apps/web/src/**/*.test.ts` | ✓ |
| Typ-/Lint | svelte-check, eslint, clippy | – | ✓ |
| Container-Smoke | `scripts/smoke-compose.sh` | – | ✓ (amd64) |
| E2E/UI | Playwright 1.63 (Chromium) gegen Compose-Stack, Mock-Provider auf dem Host | `tests/e2e`, `scripts/e2e.sh` | ✓ |
| Contract (Protect) | synthetischer Controller + versionierte Fixtures | `tests/contracts` (M030) | geplant |
| Nativ | freigegebener Testplan, autorisierte Umgebung | `docs/compatibility/evidence` | **nie in CI** |

## Grundsätze

- Jede Funktion: Positiv- **und** Negativtests (ungültige Zustände, Ausfälle, Fehlkonfiguration).
- Fixtures sind ausschließlich künstlich; kein fremdes Kamera-/Personenmaterial.
- Ergebnisse werden ehrlich berichtet; übersprungene Tests werden als übersprungen genannt.
- `synthetic_verified` ≠ `native_verified`.

## Native Tests

Vorlage: `NATIVE-TESTPLAN-TEMPLATE.md`, Pläne unter `plans/` (TP-001 Adoption als Entwurf), Nachweise unter `docs/compatibility/evidence/`.

### Kurzform (Freigabe erforderlich)

Ziel · Umgebung (Konsole, Protect-Version, Kamera+FW, Netz/VLAN) · Vorbedingungen & Rückweg
(Deaktivierung, Entfernen des Geräts) · Schritte · erwartete Beobachtung inkl. Readback ·
Datenschutz (keine Fremdpersonen im Bild) · Abbruchkriterien · Freigabe (Name, Datum).
