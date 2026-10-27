# Risikoentscheidung: Geräteimitation / Interoperabilität

> Datum: 2026-10-26 · Entscheider: Projektverantwortlicher (Interview) · Bezug: Q-002, AKL-M000-06
> **Keine Rechtsberatung.** Dies dokumentiert eine bewusste Entscheidung des Projektverantwortlichen.

## Entscheidung

Der Projektverantwortliche trägt das rechtliche Restrisiko der Emulation eines AI-Key-/AI-Port-
Geräts gegenüber UniFi Protect **bewusst selbst**, unter folgenden Bedingungen:

1. Nutzung ausschließlich mit **eigenen** Geräten, Konten und Netzwerken (privater Betrieb).
2. Keine Umgehung von Lizenzprüfungen, Kopierschutz, Zugriffssicherungen oder Cloud-Diensten des Herstellers.
3. ~~Kein Dekompilieren/Extrahieren von Firmware~~ – **geändert 2026-10-27 (ADR-005):** statische
   Analyse öffentlich angebotener AI-Key-Firmware ist erlaubt; Schlüssel, Zugangsdaten und Modelle
   werden weiterhin weder extrahiert noch verwendet, Firmware(-teile) nicht ins Repository übernommen.
4. Keine Werbung mit Herstellermarken; durchgehende Kennzeichnung „inoffiziell · unabhängig“.
5. Native Live-Tests nur nach freigegebenem Testplan (docs/testing/STRATEGY.md).

## Ergänzung 2026-10-27: Firmware-Analyse und öffentliches Repository

Der Projektverantwortliche hat die statische Firmware-Analyse freigegeben (ADR-005) und entschieden,
das Repository **öffentlich** zu lassen. Er trägt das Restrisiko ausdrücklich auch dafür, dass aus der
Analyse abgeleitete Erkenntnisse und Implementierungen öffentlich einsehbar sind.

## Geltungsgrenzen

- Gilt **nicht** automatisch für eine öffentliche Verbreitung von Releases (M120). Vor einem
  öffentlichen Stable-Release wird die Entscheidung erneut vorgelegt.
- Gilt nicht für Dritte, die AI-Keyless betreiben; README und UI weisen auf Eigenverantwortung hin.

## Status

AKL-M000-06 ist damit als Risikoentscheidung abgeschlossen. Native Tests bleiben trotzdem
blockiert, weil noch **keine Testumgebung** existiert (Q-003).
