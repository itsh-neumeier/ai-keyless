# ADR-005: Statische Analyse öffentlich angebotener AI-Key-Firmware

- **Status:** Akzeptiert · **Datum:** 2026-10-27 · **Entscheider:** Projektverantwortlicher
- **Ersetzt teilweise:** ADR-002 Punkt 3 („Firmware-Images“), RISK-ACCEPTANCE Bedingung 3
- **Keine Rechtsberatung.**

## Kontext

Das Discovery-, Adoptions- und Kontrollverhalten eines AI Key ist öffentlich nicht dokumentiert
(Q-006, Q-007, Q-008). Black-Box-Tests an der eigenen UNVR zeigen nur die Controller-Seite; was
Protect von einem Gerät erwartet, bleibt offen. Der Projektverantwortliche hat am 2026-10-27
entschieden, die vom Hersteller öffentlich zum Download angebotene AI-Key-Firmware statisch zu
analysieren („Bitte Regel aufheben, es wird nur im Homelab-Umfeld genutzt“) und das Repository
trotzdem öffentlich zu lassen („Öffentlich lassen, Risiko tragen“).

## Entscheidung

**Erlaubt** (Zweck: Interoperabilität von AI-Keyless mit der eigenen Protect-Installation):

1. Herunterladen öffentlich angebotener Firmware vom Update-Server des Herstellers.
2. Lokale **statische** Analyse: Entpacken, Dateilisten, Konfigurationen, Skripte, Zeichenketten,
   Protokollkonstanten, Ports, Endpunkte, Datenformate, Abläufe.
3. Dokumentation der Erkenntnisse **in eigenen Worten** als Fakten unter
   `docs/compatibility/FIRMWARE-NOTES.md`, je Aussage mit Firmware-Version und SHA-256 des Images.

**Weiterhin verboten:**

1. Extrahieren, Speichern oder Verwenden von privaten Schlüsseln, Zertifikaten, Passwörtern,
   Tokens oder sonstigen Zugangsdaten aus der Firmware. AI-Keyless nutzt nur eigene Identitäten
   (`crates/device-lifecycle`, AKL-M040-01).
2. Firmware, entpackte Dateien oder Teile davon (Binärdateien, Skripte, Konfigurationen, Webassets,
   Modellgewichte) ins Repository übernehmen oder weiterverbreiten.
3. Herstellercode kopieren, portieren oder „nachtippen“; Implementierung bleibt eigenständig.
4. Umgehen von Lizenzprüfungen, Signaturprüfungen, Kopierschutz oder Zugriffssicherungen; Ausführen
   der Firmware gegen Herstellerdienste/Cloud.
5. Code aus `Olli0103/unifi-ai-key-emulator` (unverändert NO-GO, ADR-002).

**Ablage:** Firmware und Analyse-Rohdaten nur außerhalb des Repositorys (`D:\ai-keyless-firmware\`
auf dem Lab-PC), nicht versioniert.

**Evidenz:** Statische Erkenntnisse sind `needs_evidence` – sie beschreiben Herstellerverhalten,
belegen aber keine Fähigkeit von AI-Keyless. `native_verified` weiterhin nur per Testplan mit Readback.

## Erweiterung 2026-10-27: Software der eigenen Konsole

Auf Entscheidung des Projektverantwortlichen („Ja, laden und prüfen“) gilt dieselbe Regelung für
Firmware/Software der **eigenen** Konsole (UniFi OS und darin enthaltenes UniFi Protect), um zu
verstehen, welche Discovery-Antworten und Adoptionsdaten Protect von einem AI-Key-Gerät erwartet.
Gleiche Grenzen wie oben; signierte Download-Links werden nicht gespeichert.

## Konsequenzen

- Schnellere, gezieltere Hypothesen für Discovery/Adoption/Kontrollkanal.
- **Clean-Room-Trennung (AGENTS.md) entfällt für diese Komponenten:** dieselbe Sitzung analysiert
  und implementiert. Wird in betroffenen PRs vermerkt.
- **Rechtliches Restrisiko** (Urheberrecht, Herstellerbedingungen) trägt der Projektverantwortliche,
  ausdrücklich auch für die öffentliche Sichtbarkeit des Repositorys (`RISK-ACCEPTANCE.md`).
- Vor einem öffentlichen Release (M120) erneut prüfen.
