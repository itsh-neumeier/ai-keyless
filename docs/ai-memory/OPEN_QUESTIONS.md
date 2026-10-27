# Offene Fragen und benötigte Nachweise

| ID | Frage | Wer entscheidet / wie belegen | Blockiert |
|---|---|---|---|
| ~~Q-001~~ | ✅ Beantwortet 2026-10-26: AGPL-3.0-or-later (ADR-004) | – | – |
| ~~Q-002~~ | ✅ 2026-10-26: Risiko bewusst getragen, privater Betrieb (RISK-ACCEPTANCE.md); vor öffentlichem Release erneut vorlegen | Projektverantwortlicher | – |
| Q-003 | Testumgebung: **2026-10-26 noch keine vorhanden.** Erneut fragen, sobald M040-04 synthetisch verifiziert ist (Konsole, Protect-Version, Kameras, Test-VLAN) | Projektverantwortlicher | alle `native_verified`-Tests |
| Q-004 | Offizielle AI-Key-FAQ (help.ui.com) – zurückgestellt (Entscheidung 2026-10-26: später) | Projektverantwortlicher fügt Text später ein | Matrix-Vervollständigung |
| Q-005 | Teilweise beantwortet 2026-10-26: **Draft-PR erlaubt.** Issues/Meilensteine anlegen: noch nicht freigegeben | Projektverantwortlicher | Issue-basierte Steuerung |
| Q-006 | Discovery-Vertrag (UDP, Ports, Payload) | Black-Box-Test in autorisierter Umgebung | M040-03 |
| Q-007 | Kontrollkanal-Vertrag (berichtet: „UCP4“ über WebSocket) | Black-Box-Test | M040-04 |
| Q-008 | Caption-Callback-Vertrag und Readback-Pfad je Protect-Version | Black-Box-Test mit Readback | M070 |
| Q-009 | Nativer Such-Vertrag: legacy CLIP vs. E5 (384 Dim.) | Black-Box-Test | M090 |
| Q-010 | Netzwerkmodus für NAS (host vs. macvlan) inkl. Portainer | Messung | M080-01 |
| Q-011 | Provider: ✅ Ollama, OpenAI-kompatibel lokal, OpenAI Cloud, weitere Cloud, Coral TPU. **Offen: Kostenrahmen für Cloud-Aufrufe** | Projektverantwortlicher | M060-07 |
| Q-012 | Herstellerangabe 1.000 vs. 1.800 Detektionen/h | Herstellerdoku/Test | Lastziele M080 |
| Q-013 | Welche weiteren Cloud-Provider („Andere Cloud“ gewählt, aber nicht benannt)? | Projektverantwortlicher | M060-08 |
| Q-014 | Konkretes GPU-Modell/VRAM (NVIDIA < 12 GB) und Intel/AMD-Modell für Messungen | Projektverantwortlicher | M080-06 |
