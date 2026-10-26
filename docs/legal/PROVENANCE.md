# Provenienz und Rechtelage

> Stand: 2026-10-26 · Verantwortlich: Projekt-Lead (Claude Code) · Freigabe: Projektverantwortlicher
> Status: **verbindlich** – jede neue externe Quelle wird hier vor der Nutzung eingetragen.

## 1. Grundsatz

AI-Keyless wird als **eigenständige Clean-Room-Implementierung** entwickelt (ADR-002).
Es wird **kein** fremder Quellcode, keine proprietäre Firmware, keine Schlüssel/Zertifikate,
keine Modellgewichte und keine Webassets/Icons/CSS Dritter übernommen, kopiert, portiert,
vendored oder geforkt, solange keine dokumentierte, belastbare Rechtefreigabe vorliegt.

AI-Keyless ist ein **inoffizielles, unabhängiges** Projekt und weder von Ubiquiti entwickelt,
zertifiziert noch unterstützt. „UniFi“, „UniFi Protect“, „AI Key“ und „AI Port“ sind Marken
ihrer Inhaber und werden ausschließlich beschreibend zur Angabe der Kompatibilität verwendet.

## 2. Referenzprojekt `Olli0103/unifi-ai-key-emulator`

| Feld | Wert (geprüft 2026-10-26) |
|---|---|
| URL | https://github.com/Olli0103/unifi-ai-key-emulator |
| Letzter Commit (`main`) | `4c71b732723ce2ccaec0a540736675789db9173d` – 2026-09-24T10:34:18+02:00 – „Recover accepted AI Port streams after decoder exit (#122)“ |
| Lizenzdatei im Wurzelverzeichnis | **keine** (`LICENSE`, `LICENSE.md`, `COPYING`, `NOTICE` → HTTP 404; Tree-Listing ohne Lizenzdatei) |
| Eigene Lizenzaussage (README) | „no license“; Lizenz, Provenienz und Release-Governance sind laut Projekt „open product work“; PLAN.md nennt Apache-2.0 nur als **Kandidaten** |
| Ermittlungsmethode | `git ls-remote`; Commit-Metadaten per `git fetch --depth 5 --filter=blob:none` (**ohne Dateiinhalte**); öffentliche Markdown-Dokumente (README, PLAN, BUILD-RESULT, docs/planning/issues.md) gelesen |

**Bewertung:** Ohne Lizenz gilt das Urheberrecht des Autors uneingeschränkt. Öffentliche
Sichtbarkeit ist **keine** Erlaubnis zur Übernahme, Veränderung oder Weiterveröffentlichung.
→ **Nutzung des Quellcodes: NO-GO.** Zulässig ist nur das Lesen der öffentlichen
Dokumentation als Quelle für *Fakten* (z. B. „Adoption auf Protect 7.3.56 beobachtet“),
nicht für Ausdruck, Struktur oder Code.

### Was aus der Referenz verwendet wird (nur Fakten/Ideen, keine Ausdrucksform)

| Information | Quelle | Verwendung in AI-Keyless |
|---|---|---|
| Berichtete Evidenz (Adoption 7.3.56, Captions 7.3.60, G5 Flex/G4 Instant) | README, BUILD-RESULT | Nur als `needs_evidence`-Hypothesen in `docs/compatibility/MATRIX.md`; **nicht** als eigene Evidenz |
| Konzept „HTTP 200 ≠ Persistenz“ (`http_accepted`) | README | Eigenständig formuliertes Prinzip (ist ohnehin Projektregel) |
| 12-Jobs-pro-Stunde-Testbegrenzung | README, PLAN | Als anfängliche Sicherheitsvorgabe dokumentiert |
| Getrennte Profile AI Key / AI Port | PLAN | Eigenständig in ADR-003 begründet |

Es wurde **keine Quelldatei** des Referenzprojekts geöffnet, heruntergeladen oder gelesen.

## 3. Offene Aufgabe: Lizenzanfrage (nicht versendet)

Eine Kontaktaufnahme mit dem Autor des Referenzprojekts zur Klärung einer Lizenz bzw.
Nutzungsfreigabe ist als Backlog-Aufgabe **AKL-M000-04** dokumentiert. Sie wird **nicht**
ohne ausdrückliche Freigabe des Projektverantwortlichen und nicht in dessen Namen versendet.
Entwurf: siehe `docs/legal/LICENSE-REQUEST-DRAFT.md`.

Selbst bei späterer Freigabe gilt ADR-002 Abschnitt „Migration“: zuerst Bestands- und
Vertragstests, dann schrittweise Migration hinter Adaptern, keine Regression der Adoption.

## 4. Eigene Lizenz von AI-Keyless

**Noch nicht festgelegt** – Entscheidung des Projektverantwortlichen (OPEN_QUESTIONS Q-001).
Bis dahin: `license = "LicenseRef-AI-Keyless-Pending"` in Cargo, keine Veröffentlichung als
Open Source, keine GHCR-Releases.

## 5. Drittabhängigkeiten

Alle Laufzeit- und Build-Abhängigkeiten stammen aus öffentlichen Registries (crates.io, npm,
Docker Hub/gcr.io) und sind über `Cargo.lock` bzw. `apps/web/pnpm-lock.yaml` gepinnt.
Eine Lizenzinventur (z. B. `cargo deny check licenses`) ist Backlog **AKL-M120-02**.

| Komponente | Herkunft | Lizenz (lt. Registry) |
|---|---|---|
| Rust: tokio, axum, tower-http, serde, sqlx, tracing | crates.io | MIT / Apache-2.0 |
| Web: SvelteKit 3.1, Svelte 5, Vite 8, Tailwind CSS 4 | npm | MIT |
| Container: postgres:17-alpine, distroless/cc-debian12 | Docker Hub / gcr.io | PostgreSQL / Apache-2.0 (+ Debian-Pakete) |

## 6. Eigene Assets

Logo (`apps/web/static/favicon.svg`), Icons (`apps/web/src/lib/components/icons.ts`) und alle
Design-Tokens wurden für dieses Projekt neu erstellt. Keine Hersteller-Icons, -Logos oder
-Stylesheets.

## 7. Änderungsprotokoll

| Datum | Änderung |
|---|---|
| 2026-10-26 | Erstfassung (AKL-M000) |
