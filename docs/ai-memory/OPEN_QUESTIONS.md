# Offene Fragen und benötigte Nachweise

| ID | Frage | Wer entscheidet / wie belegen | Blockiert |
|---|---|---|---|
| Q-001 | Welche Lizenz erhält AI-Keyless (z. B. Apache-2.0, AGPL-3.0, proprietär)? | Projektverantwortlicher | Release, Veröffentlichung (M120), GHCR |
| Q-002 | Rechtliche Bewertung Geräteimitation/Interoperabilität (DE/EU, Hersteller-AGB) | Projektverantwortlicher, ggf. Rechtsberatung | Native Live-Tests (M040-05) |
| Q-003 | Welche autorisierte Testumgebung (Konsole, Protect-Version, Kameras) steht bereit? Isoliertes VLAN? | Projektverantwortlicher | alle `native_verified`-Tests |
| Q-004 | Offizielle AI-Key-FAQ (help.ui.com) – Funktionen, Limits, Mindestversionen | manuell sichten (aus Cloud-Umgebung HTTP 403) | Matrix-Vervollständigung |
| Q-005 | Freigabe: GitHub-Issues anlegen (`APPLY=1 scripts/import-backlog.sh`) und PR öffnen? | Projektverantwortlicher | AKL-M010-06, Issue-basierte Steuerung |
| Q-006 | Discovery-Vertrag (UDP, Ports, Payload) | Black-Box-Test in autorisierter Umgebung | M040-03 |
| Q-007 | Kontrollkanal-Vertrag (berichtet: „UCP4“ über WebSocket) | Black-Box-Test | M040-04 |
| Q-008 | Caption-Callback-Vertrag und Readback-Pfad je Protect-Version | Black-Box-Test mit Readback | M070 |
| Q-009 | Nativer Such-Vertrag: legacy CLIP vs. E5 (384 Dim.) | Black-Box-Test | M090 |
| Q-010 | Netzwerkmodus für NAS (host vs. macvlan) inkl. Portainer | Messung | M080-01 |
| Q-011 | Bevorzugte Modell-Provider (lokal Ollama vs. extern) und Kostenrahmen | Projektverantwortlicher | M060 |
| Q-012 | Herstellerangabe 1.000 vs. 1.800 Detektionen/h | Herstellerdoku/Test | Lastziele M080 |
