# Offene Fragen und benötigte Nachweise

| ID | Frage | Wer entscheidet / wie belegen | Blockiert |
|---|---|---|---|
| ~~Q-001~~ | ✅ Beantwortet 2026-10-26: AGPL-3.0-or-later (ADR-004) | – | – |
| ~~Q-002~~ | ✅ 2026-10-26: Risiko bewusst getragen, privater Betrieb (RISK-ACCEPTANCE.md); vor öffentlichem Release erneut vorlegen | Projektverantwortlicher | – |
| Q-003 | Testumgebung: ✅ UNVR unter 192.168.10.181 (Heimnetz). Lab-PC: Windows, `D:\GitHub\ai-keyless`, Docker Desktop/WSL2. TP-000 freigegeben (2026-10-27). Aus der Cloud-Umgebung **nicht erreichbar** (Egress-Gateway antwortet). Native Tests nur mit AI-Keyless im LAN + Claude-Sitzung auf dem PC (Remote Control) oder HTTPS-Tunnel (nur lesend). Offen: Protect-Version der UNVR, Entscheidung zum Zugriffsweg | Projektverantwortlicher | `native_verified`-Tests |
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
