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

**AGPL-3.0-or-later** (Entscheidung des Projektverantwortlichen vom 2026-10-26, ADR-004).
Volltext in `LICENSE` (offizielle Fassung von https://www.gnu.org/licenses/agpl-3.0.txt,
abgerufen 2026-10-26, SHA-256 `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`).
Rechtliche Risikoentscheidung zur Geräteimitation: `docs/legal/RISK-ACCEPTANCE.md`.

## 4a. Öffentliche Protokollbeschreibungen (nur Fakten)

Genutzt werden ausschließlich **Fakten** (Portnummer, Bytefolgen, Feldbedeutungen). Es wird kein
Code, keine Struktur und kein Ausdruck übernommen; die Implementierung in
`crates/protect-compat/src/discovery.rs` ist eigenständig geschrieben. Alle daraus abgeleiteten
Aussagen sind `needs_evidence`, bis ein eigener Testplan sie bestätigt (TP-002).

| Quelle | Abgerufen | Lizenz/Status | Genutzte Fakten |
|---|---|---|---|
| Nmap-Skript `ubiquiti-discovery.nse`, https://svn.nmap.org/nmap/scripts/ubiquiti-discovery.nse (Autor laut Datei: Tom Sellers) | 2026-10-27 | Nmap Public Source License („Same as Nmap“) – **kein Code übernommen** | UDP 10001; Anfragen v1 `01 00 00 00`, v2 `02 08 00 00`; Antwortkopf Version/Kommando/Länge (2 Byte BE); TLV Typ 1 Byte, Länge 2 Byte BE; Typbedeutungen (0x01, 0x02, 0x03, 0x0a–0x0d, 0x0f, 0x14–0x17) |
| Wireshark Display-Filter-Referenz „ubdp“, https://www.wireshark.org/docs/dfref/u/ubdp.html | 2026-10-27 | öffentliche Dokumentation | Feldbezeichnungen und -typen des Ubiquiti Discovery Protocol (Wireshark 3.0.0–4.6.9) |

## 4b. Hersteller-Firmware (statische Analyse, ADR-005)

| Quelle | Abgerufen | Status | Nutzung |
|---|---|---|---|
| AI-Key-Firmware vom Update-Server des Herstellers, `https://fw-update.ui.com/api/firmware/f966bb8d-3c4f-438a-91aa-788be18911f1/data` (Link vom Projektverantwortlichen, 2026-10-27) | 2026-10-27 | proprietär; nur lokal unter `D:\ai-keyless-firmware\`, **nicht** im Repository | Statische Analyse; Erkenntnisse in eigenen Worten in `docs/compatibility/FIRMWARE-NOTES.md` (mit Version und SHA-256). Keine Schlüssel, kein Code, keine Dateien übernommen. |
| AI-Key-Firmware v2.0.4 (Vollabbild), `https://fw-update.ui.com/api/firmware/13efe860-69b4-4f1f-b75d-34713bfb499e/data`, SHA-256 laut Hersteller `442a6ad0…2b5f50` | 2026-10-27 | proprietär; nur lokal | wie oben |
| UniFi OS v6.0.11+dd9ab55 (Plattform UNVRINS) der eigenen Konsole, `https://fw-update.ui.com/api/firmware/7a682b92-00d9-4f49-b35e-8a0530409a71/data`, SHA-256 laut Hersteller `0ad05ae6…458ad6` (ein zuvor genannter signierter Link war abgelaufen und wird nicht gespeichert) | 2026-10-27 | proprietär; nur lokal unter `D:\ai-keyless-firmware\unvr\` | Prüfen, ob/welches Protect enthalten ist; Controller-Erwartungen an Discovery/Adoption (ADR-005, Erweiterung) |

## 5. Drittabhängigkeiten

Alle Laufzeit- und Build-Abhängigkeiten stammen aus öffentlichen Registries (crates.io, npm,
Docker Hub/gcr.io) und sind über `Cargo.lock` bzw. `apps/web/pnpm-lock.yaml` gepinnt.
Eine Lizenzinventur (z. B. `cargo deny check licenses`) ist Backlog **AKL-M120-02**.

| Komponente | Herkunft | Lizenz (lt. Registry) |
|---|---|---|
| Rust: tokio, axum, tower-http, serde, sqlx, tracing | crates.io | MIT / Apache-2.0 |
| Web: SvelteKit 3.1, Svelte 5, Vite 8, Tailwind CSS 4 | npm | MIT |
| E2E (nur Test): `@axe-core/playwright` 4.13.0 inkl. `axe-core` | npm | MPL-2.0 (unverändert genutzt, nicht ausgeliefert) |
| Container: postgres:17-alpine, distroless/cc-debian12 | Docker Hub / gcr.io | PostgreSQL / Apache-2.0 (+ Debian-Pakete) |

## 6. Eigene Assets

Logo (`apps/web/static/favicon.svg`), Icons (`apps/web/src/lib/components/icons.ts`) und alle
Design-Tokens wurden für dieses Projekt neu erstellt. Keine Hersteller-Icons, -Logos oder
-Stylesheets.

## 7. Änderungsprotokoll

| Datum | Änderung |
|---|---|
| 2026-10-26 | Erstfassung (AKL-M000) |
| 2026-10-26 | Lizenz AGPL-3.0-or-later festgelegt; Risikoentscheidung ergänzt |
| 2026-10-27 | Abschnitt 4a: öffentliche Protokollbeschreibungen zur Discovery (Nmap, Wireshark) als Faktenquellen |
