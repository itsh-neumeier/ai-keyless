# ADR-004: Projektlizenz AGPL-3.0-or-later

- **Status:** Akzeptiert · **Datum:** 2026-10-26 · **Entscheider:** Projektverantwortlicher (Interview 2026-10-26) · **Meilenstein:** AKL-M000-05

## Kontext

Ohne Lizenz sind Veröffentlichung, Beiträge Dritter und GHCR-Releases blockiert (Q-001).
AI-Keyless ist eine Netzwerkanwendung (Admin-UI, Management-API); klassisches GPL-Copyleft
greift bei reinem Netzwerkbetrieb nicht.

## Entscheidung

AI-Keyless steht unter der **GNU Affero General Public License v3.0 oder später**
(SPDX `AGPL-3.0-or-later`). Volltext: `LICENSE` (offizielle Fassung von gnu.org,
SHA-256 `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`).

Umsetzung:
- `license` in `Cargo.toml` (Workspace) und `apps/web/package.json`, OCI-Label im Image.
- **AGPL §13 (Quellcode für Netzwerknutzer):** Die API liefert `source_url`
  (`CARGO_PKG_REPOSITORY`, wird beim Fork mit angepasst), die UI verlinkt ihn unter
  *System → Über AI-Keyless*. Wer eine veränderte Fassung betreibt, muss `repository` in
  `Cargo.toml` auf die eigene Quelle setzen.
- „or later“ folgt der Empfehlung der FSF; ein Wechsel auf `AGPL-3.0-only` ist bis zur
  ersten öffentlichen Veröffentlichung ohne Weiteres möglich.

## Konsequenzen

- Abhängigkeiten müssen AGPL-kompatibel sein (MIT, Apache-2.0, BSD, MPL-2.0 ✓; proprietäre oder
  „GPL-2.0-only“-Abhängigkeiten ✗). Prüfung per `cargo deny`/Lizenzinventur (AKL-M120-02).
- Beiträge Dritter stehen automatisch unter AGPL (inbound = outbound); ein CLA ist nicht vorgesehen.
- Eine spätere Übernahme von Code aus dem Referenzprojekt wäre nur möglich, wenn dessen künftige
  Lizenz AGPL-kompatibel ist (z. B. Apache-2.0) – unverändert nur nach ADR-002-Prozess.
- Modelle/Gewichte werden nicht mitgeliefert; ihre Lizenzen bleiben Sache des Betreibers.
