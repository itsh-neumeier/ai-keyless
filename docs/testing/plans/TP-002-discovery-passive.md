# TP-002: Discovery – passiver Empfang im LAN

| Feld | Inhalt |
|---|---|
| Testplan-ID | TP-002 |
| Backlog-Issue | AKL-M030-03 (#10), Vorbereitung AKL-M040-03 (#5) |
| Ziel | Prüfen, ob und wie die UNVR Discovery-Anfragen ins LAN sendet; Hypothesen DISC-H01…H04 (`tests/contracts/discovery/hypotheses.json`) bestätigen oder widerlegen |
| Erwartete Evidenzstufe danach | Für die **Anfrageseite** der beobachteten Version: bestätigt/widerlegt (Beobachtung, kein Fähigkeitsnachweis). Kein `native_verified` für AI-Keyless (es antwortet nicht). |
| Status | **freigegeben**, Lauf 1 gestartet 2026-10-27 |
| Freigabe (Name, Datum) | Projektverantwortlicher, 2026-10-27 (Chat: „Freigabe hast du“) |

## Umgebung

| Element | Wert |
|---|---|
| Konsole | UNVR 192.168.10.181, **UniFi OS 6.0.11**, **Protect 7.2.105** (Update auf 7.3.70 angeboten, noch nicht eingespielt), 8 × G5 Turret Ultra FW 5.5.31 – abgelesen vom Projektverantwortlichen 2026-10-27 |
| PC | Windows 11, 192.168.10.160/24 (gleiches Segment wie die UNVR) |
| Werkzeug | `scripts/discovery-listen.ps1` (nur Empfang, sendet nichts) |

## Ablauf

| # | Aktion | Wer | Erwartete Beobachtung |
|---|---|---|---|
| 1 | `scripts\discovery-listen.ps1 -Minutes 10` starten; Windows-Firewall-Abfrage nur für **private** Netze erlauben | Claude/Projektverantwortlicher | „Lausche auf UDP 10001“ |
| 2 | 10 Minuten nichts tun (Grundrauschen: sendet die UNVR periodisch?) | – | 0…n Datagramme |
| 3 | Zweiter Lauf: in Protect die Ansicht zum Hinzufügen/Suchen von Geräten öffnen und **ohne etwas zu adoptieren** wieder schließen | Projektverantwortlicher | ggf. Anfragen von 192.168.10.181 |
| 4 | Log auswerten: Absender, Ziel (Broadcast?), Kopf, Länge, TLVs | Claude | Vergleich mit DISC-H01…H04 |

## Ausdrücklich nicht erlaubt

- Senden von Antworten oder eigenen Suchanfragen, Port-Scans, Paketmitschnitt mit pcap/Wireshark.
- Anmeldung an der UNVR durch Claude, Adoption, Änderungen an Einstellungen oder Kameras.
- Rohlog im Repository (enthält ggf. MAC-Adressen/Seriennummern) – nur redigierte Auswertung.

## Ergebnis

Ablage: `docs/compatibility/evidence/<Datum>-tp002-discovery-passiv.md` (redigiert). Danach Evidenzstufen in
`hypotheses.json` anpassen (bestätigt → Beobachtung mit Version; widerlegt → `unsupported` für diese Version).
Was die UNVR als Antwort von einem AI-Key-Gerät erwartet (DISC-H07), klärt TP-002 **nicht**.

## Rückweg

Skript endet nach Ablauf selbst; Firewall-Regel für PowerShell bei Bedarf in den Windows-Einstellungen entfernen.
