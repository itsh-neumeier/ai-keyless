# Offene Fragen und benötigte Nachweise

| ID | Frage | Wer entscheidet / wie belegen | Blockiert |
|---|---|---|---|
| ~~Q-001~~ | ✅ Beantwortet 2026-10-26: AGPL-3.0-or-later (ADR-004) | – | – |
| ~~Q-002~~ | ✅ 2026-10-26: Risiko bewusst getragen, privater Betrieb (RISK-ACCEPTANCE.md); vor öffentlichem Release erneut vorlegen | Projektverantwortlicher | – |
| Q-003 | Testumgebung: ✅ UNVR unter 192.168.10.181 (Heimnetz). Lab-PC: Windows, `D:\GitHub\ai-keyless`, Docker Desktop/WSL2. TP-000 durchgeführt (2026-10-27, `evidence/2026-10-27-tp000-umgebung.md`): ✅ **UniFi OS 5.1.33, Protect 7.2.105**, 8 × G5 Turret Ultra (FW 5.5.31), Produktivbetrieb. Aus der Cloud-Umgebung **nicht erreichbar**; Live-Tests über Claude-Sitzung auf dem PC (Remote Control, erprobt). Offen: isoliertes Test-VLAN vs. Produktivnetz, Update auf Protect 7.3.70/UniFi OS 6.0.11 vor oder nach TP-001, Unterstützt 7.2.105 AI-Key überhaupt? | Projektverantwortlicher | `native_verified`-Tests |
| Q-004 | Offizielle AI-Key-FAQ (help.ui.com) – zurückgestellt (Entscheidung 2026-10-26: später) | Projektverantwortlicher fügt Text später ein | Matrix-Vervollständigung |
| ~~Q-005~~ | ✅ 2026-10-27: Agent darf im Repo agieren wie nötig (Issues, PRs; Merge nach Freigabe) | – | – |
| Q-006 | Discovery-Vertrag (UDP, Ports, Payload) | Black-Box-Test in autorisierter Umgebung | M040-03 |
| Q-007 | Kontrollkanal-Vertrag (berichtet: „UCP4“ über WebSocket) | Black-Box-Test | M040-04 |
| Q-008 | Caption-Callback-Vertrag und Readback-Pfad je Protect-Version | Black-Box-Test mit Readback | M070 |
| Q-009 | Nativer Such-Vertrag: legacy CLIP vs. E5 (384 Dim.) | Black-Box-Test | M090 |
| Q-010 | Netzwerkmodus für NAS (host vs. macvlan) inkl. Portainer | Messung | M080-01 |
| Q-011 | Provider: ✅ Ollama, OpenAI-kompatibel lokal, OpenAI Cloud, weitere Cloud, Coral TPU. **Offen: Kostenrahmen für Cloud-Aufrufe** | Projektverantwortlicher | M060-07 |
| Q-012 | Herstellerangabe 1.000 vs. 1.800 Detektionen/h | Herstellerdoku/Test | Lastziele M080 |
| Q-013 | Welche weiteren Cloud-Provider („Andere Cloud“ gewählt, aber nicht benannt)? | Projektverantwortlicher | M060-08 |
| ~~Q-014~~ | ✅ NVIDIA T600 (4 GB VRAM) und Coral Edge TPU (USB) am PC des Projektverantwortlichen | – | M080-06, M100-02 |
